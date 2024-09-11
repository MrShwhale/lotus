const MAX_RESULTS = 5;
const WIKI_PREFIX = "https://scp-wiki.wikidot.com/";

let isRequesting = false;

let bans = undefined;
let tagStrings = [];

// TODO store banned names for unban list
if (typeof (Storage) !== "undefined") {
    // Put a placeholder value in if there are no existing bans
    if (!localStorage.getItem("bans")) {
        window.localStorage.setItem("bans", "[]");
    }

    // TODO add ensuring that all these are valid pids, removing others
    bans = JSON.parse(window.localStorage.getItem("bans"));
}
else {
    console.log("No localStorage support. Bans will not be saved.");
}

// Takes a value, array, start/end indicies (inclusive), and compare function
// Returns the correct index to insert the value at, or -1 if start < end.
// If the value is already in the list, inserts before the first existing value
function binarySearchIndex(value, array, start, end, compare) {
    let test = false;
    let mid = -1;

    while (start <= end) {
        mid = Math.floor((end + start) / 2);

        test = compare(value, array[mid]) <= 0;

        if (test) {
            end = mid - 1;
        }
        else {
            start = mid + 1;
        }
    }

    if (!test) {
        mid += 1;
    }

    return mid;
}

// Retrieves and displays recommendations for the given user with the given settings
// TODO add more rate limiting
async function showRecs() {
    if (isRequesting) {
        console.log("Already requesting!")
        return;
    }

    isRequesting = true;

    let username = document.getElementById("user-search").value;
    let url = window.location.href + "rec";
    url += "?user=" + encodeURIComponent(username);

    if (tagStrings.length > 0) {
        url += "&tags=";
        for (const tag of tagStrings) {
            let tagId = tags.indexOf(tag).toString();
            if (tagId == "-1") {
                continue;
            }

            url += tagId;
            url += "+";
        }
        // Get rid of trailing plus 
        url = url.substring(0, url.length - 1);
    }

    if (bans.length > 0) {
        url += "&bans=";
        for (const ban of bans) {
            // If this is not a valid int, ignore for now. It will be removed on reload
            // Does not consider numbers too large to be pids
            if (ban != Number.parseInt(ban)) {
                continue;
            }

            url += ban;
            url += "+"
        }
        url = url.substring(0, url.length - 1);
    }

    try {
        document.getElementById("rec-container-container").classList.remove("hidden");

        let recommendationsHolder = document.getElementById("rec-container");

        // Clear of previous recs/errors
        recommendationsHolder.innerHTML = "";

        let loadingElement = document.createElement("div");
        loadingElement.classList.add("response-text");
        loadingElement.innerHTML = "LOADING REQUESTS...";

        recommendationsHolder.appendChild(loadingElement);

        const response = await fetch(url);
        if (!response.ok) {
            console.log(response);

            recommendationsHolder.innerHTML = "";

            let errorElement = document.createElement("div");
            errorElement.classList.add("response-text");
            errorElement.classList.add("error");
            errorElement.innerHTML = `ERROR ${response.status}`;

            let statusText = document.createElement("p");
            statusText.classList.add("status-text");
            statusText.innerHTML = response.statusText;

            recommendationsHolder.appendChild(errorElement);
            recommendationsHolder.appendChild(statusText);

            isRequesting = false;
            throw new Error(`Response status: ${response.status}`);
        }

        isRequesting = false;

        json = await response.json();

        recommendationsHolder.innerHTML = "";

        if ('type' in json && json.type == "error") {
            let errorElement = document.createElement("div");
            errorElement.classList.add("response-text");
            errorElement.classList.add("error");

            let statusText = document.createElement("p");
            statusText.classList.add("status-text");

            switch (json.type) {
                case "USER_PARSE_ERROR":
                    errorElement.innerHTML = "WRONG USER FORMAT";
                    statusText.innerHTML = "The username could not be loaded by the recommender.";
                    break;
                case "NO_USER":
                    errorElement.innerHTML = "USER NOT FOUND";
                    statusText.innerHTML = "A user by that name could not be found in the recommender. Have they voted enough times?";
                    break;
                case "RECOMMENDER_ERROR":
                    errorElement.innerHTML = "RECOMMENDER ERROR";
                    statusText.innerHTML = "Something went wrong with the recommendation process. Try again soon or contact an admin at wapatmore@gmail.com.";
                    break;
            }

            recommendationsHolder.appendChild(errorElement);
            recommendationsHolder.appendChild(statusText);

            throw new Error(`Valid response with error: ${json.type}`);
        }

        // Add starting dummy
        recommendationsHolder.appendChild(document.createElement("div"));

        function banId(id) {
            return (event) => {
                event.target.parentNode.remove();
                bans.push(id);
                window.localStorage.setItem("bans", JSON.stringify(bans));
            }
        }

        for (const page of json.slice(0, 30)) {
            const recHolder = document.createElement("div");
            const recLink = document.createElement("a");
            const recBan = document.createElement("button");

            recLink.innerHTML = `${page.name}`;

            recLink.setAttribute("href", WIKI_PREFIX + page.url);
            recLink.setAttribute("target", "_blank");
            recLink.classList.add("rec-link");

            recBan.addEventListener("click", banId(page.pid));
            recBan.classList.add("ban-button");

            recHolder.appendChild(recBan);
            recHolder.appendChild(recLink);
            recHolder.classList.add("rec");

            recommendationsHolder.appendChild(recHolder);
        }

        // Dummy element for functional flex formatting
        recommendationsHolder.appendChild(document.createElement("div"));

    } catch (error) {
        console.error(error.message);
    }
}

document.getElementById("search-button").addEventListener('click', () => {
    showRecs();
});

// Add everything to the tags element
// CONS actually do this with templeting in the final
let tagContainer = document.getElementById("tag-container");
for (const tag of tags) {
    let newTag = document.createElement("button");
    newTag.innerHTML = tag;
    newTag.classList.add("tag")
    tagContainer.appendChild(newTag);
}

function toggleTag(event) {
    let selected = event.target.classList.contains("selected-tag");

    if (selected) {
        // Remove from the tags list
        let index = tagStrings.indexOf(event.target.innerHTML);
        tagStrings.splice(index, 1);
    }
    else {
        tagStrings.push(event.target.innerHTML);
    }

    event.target.classList.toggle("selected-tag");

    // Take out the element so the indices are correct
    event.target.remove();

    // TODO Make sure this is consistent with other binary search (who cares?)
    let low = 0;
    let high = tagContainer.children.length - 1;

    // If this WAS selected, it shouldn't be anymore
    if (selected) {
        low = tagStrings.length;
        high = tagContainer.children.length - 1;
    }
    else {
        low = 0;
        high = tagStrings.length - 2;
    }

    function compareTagElements(tag1, tag2) {
        let test = tag1.innerHTML.replace("-", "").replace("_", "").localeCompare(tag2.innerHTML.replace("-", "").replace("_", ""));
        if (test == 0) {
            return tag1.innerHTML.localeCompare(tag2.innerHTML);
        }

        return test;
    }

    let index = binarySearchIndex(event.target, tagContainer.children, low, high, compareTagElements);

    tagContainer.children[index].insertAdjacentElement("beforebegin", event.target);
}

for (const tag of tagContainer.children) {
    tag.addEventListener('click', toggleTag);
}

function toggleTagPopup() {
    // Clear tag filters
    document.getElementById("tag-search").value = "";
    document.getElementById("tags-popup").classList.toggle("hidden");
}

document.getElementById("tag-select-button").addEventListener('click', toggleTagPopup);

let userSearchElement = document.getElementById("user-search");

// TODO rewrite all of this to be not bad
let acSelected = -1;

function compareNames(name1, name2) {
    return name1.localeCompare(name2, undefined, { "caseFirst": "upper" });
}

usernames.sort(compareNames);

function closeAutocomplete() {
    let acItems = document.getElementsByClassName("user-ac-holder");
    for (let item of acItems) {
        item.remove();
    }
}

function autocomplete(event) {
    let value = event.target.value;

    closeAutocomplete();

    if (!value) {
        return false;
    }

    acSelected = -1;

    let resultContainer = document.createElement("div");
    resultContainer.setAttribute("id", event.target.id + "-ac-list");
    resultContainer.classList.add("user-ac-holder");

    event.target.parentNode.appendChild(resultContainer);

    let ind = binarySearchIndex(value, usernames, 0, usernames.length - 1, compareNames);

    for (let i = ind; i < ind + MAX_RESULTS; i++) {
        let shared_substr = usernames[i].substr(0, value.length);

        // Do not overflow into users not starting with shared_substr
        if (shared_substr.toLowerCase() == value.toLowerCase()) {
            break;
        }

        let result = document.createElement("div");

        let shared = document.createElement("span");
        shared.classList.add("user-ac-shared")
        shared.innerHTML = shared_substr;

        let other = document.createElement("span");
        other.classList.add("user-ac-other")
        other.innerHTML = usernames[i].substr(value.length);

        result.appendChild(shared);
        result.appendChild(other);

        result.addEventListener("click", () => {
            userSearchElement.value = usernames[i];
            closeAutocomplete();
        });

        resultContainer.appendChild(result);
    }
}

function handlePresses(event) {
    let elementList = document.getElementById(event.target.id + "-ac-list");
    if (!elementList) {
        return;
    }
    else {
        elementList = elementList.children;
    }

    // CONS changing this slightly to look more DRY
    if (event.key == "ArrowUp" || (event.key == "Tab" && event.shiftKey)) {
        event.preventDefault();
        if (acSelected < 0) {
            acSelected = elementList;
        }
        else {
            elementList[acSelected].classList.remove("user-ac-selected");
        }

        acSelected--;
        if (acSelected < 0) {
            acSelected = elementList.length - 1;
        }

        elementList[acSelected].classList.add("user-ac-selected");
    }
    else if (event.key == "ArrowDown" || event.key == "Tab") {
        event.preventDefault();
        if (acSelected < 0) {
            acSelected = -1;
        }
        else {
            elementList[acSelected].classList.remove("user-ac-selected");
        }

        acSelected++;
        if (acSelected >= elementList.length) {
            acSelected = 0;
        }

        elementList[acSelected].classList.add("user-ac-selected");
    }
    else if (event.key == "Enter") {
        if (acSelected > -1) {
            elementList[acSelected].click();
        }
    }
}

userSearchElement.addEventListener("input", autocomplete);
userSearchElement.addEventListener("keydown", handlePresses);

document.addEventListener("click", (event) => {
    closeAutocomplete(event.target);
});


document.addEventListener("keydown", (event) => {
    if (event.key == "Enter") {
        if (!event.repeat) {
            showRecs()
        }
    }
});

document.getElementById("tag-search").addEventListener("input", (event) => {
    let input = event.target.value;
    input = input.split('')
        .join('(.{0,1})');
    let regex = new RegExp(`^(.*)${input}(.*)$`);
    for (const tag of tagContainer.children) {
        // CONS class not style attribute
        if (!regex.test(tag.innerHTML)) {
            tag.setAttribute("style", "display: none");
        }
        else {
            tag.setAttribute("style", "");
        }
    }
});

// Main TODO: 
// unban pages
// rec pagination
// fix scraper
// BUG test with d-11424 tag, no results for mr_shwhale?
