// The number of results to show when autocorrecting
const MAX_RESULTS = 5;

// Prefix of the SCP wiki
const WIKI_PREFIX = "https://scp-wiki.wikidot.com/";

let bans = undefined;
let tagStrings = [];
let recsPerPage = 30;
let isRequesting = false;
let recs = [];
let acSelected = -1;
let userSearchElement = document.getElementById("user-search");
let tagContainer = document.getElementById("tag-container");

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
async function showRecs(tags) {
    if (isRequesting) {
        console.log("Already requesting!")
        return;
    }

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

        // Filter out anything crazy
        let oldLen = bans.length;
        bans = bans.filter((item) => {
            return item.hasOwnProperty('pid') && item.hasOwnProperty('name') && item.pid == Number.parseInt(item.pid);
        });

        if (oldLen != bans.length) {
            localStorage.setItem("bans", JSON.stringify(bans));
        }

        for (const ban of bans) {
            url += ban.pid;
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

            switch (json.code) {
                case "USER_PARSE_ERROR":
                    errorElement.innerHTML = "USER NOT FOUND";
                    statusText.innerHTML = "A user by that name/id could not be found by the recommender. They may not have voted enough times by the latest scrape.";
                    break;
                case "NO_USER":
                    errorElement.innerHTML = "USER NOT SENT";
                    statusText.innerHTML = "Username was not properly sent to the server. Try again soon or create a GitHub issue if this continues";
                    break;
                case "RECOMMENDER_ERROR":
                    errorElement.innerHTML = "RECOMMENDER ERROR";
                    statusText.innerHTML = "Something went wrong with the recommendation process. Try again soon or create a GitHub issue if this continues.";
                    break;
            }

            recommendationsHolder.appendChild(errorElement);
            recommendationsHolder.appendChild(statusText);

            throw new Error(`Valid response with error: ${json.code}`);
        }

        recs = json;

        displayRecs();
    } catch (error) {
        console.error(error.message);
    }
}

function banId(id, name) {
    return (event) => {
        event.target.parentNode.remove();
        bans.push({ pid: id, name: name });
        localStorage.setItem("bans", JSON.stringify(bans));

        recs = recs.filter((value) => {
            return value.pid != id;
        });

        displayRecs();
    }
}

function displayRecs() {
    let recommendationsHolder = document.getElementById("rec-container");

    recommendationsHolder.innerHTML = "";

    // Add starting dummy for functional flex formatting
    recommendationsHolder.appendChild(document.createElement("div"));

    for (const page of recs.slice(0, recsPerPage)) {
        const recHolder = document.createElement("div");
        const recLink = document.createElement("a");
        const recBan = document.createElement("button");

        recLink.innerHTML = `${page.name}`;

        recLink.setAttribute("href", WIKI_PREFIX + page.url);
        recLink.setAttribute("target", "_blank");
        recLink.classList.add("rec-link");

        recBan.addEventListener("click", banId(page.pid, page.name));
        recBan.classList.add("ban-button");

        recHolder.appendChild(recBan);
        recHolder.appendChild(recLink);
        recHolder.classList.add("rec");

        recommendationsHolder.appendChild(recHolder);
    }

    // Add ending dummy for functional flex formatting
    recommendationsHolder.appendChild(document.createElement("div"));
}

function compareTagElements(tag1, tag2) {
    let test = tag1.innerHTML.replace("-", "").replace("_", "").localeCompare(tag2.innerHTML.replace("-", "").replace("_", ""));
    if (test == 0) {
        return tag1.innerHTML.localeCompare(tag2.innerHTML);
    }

    return test;
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

    let index = binarySearchIndex(event.target, tagContainer.children, low, high, compareTagElements);

    tagContainer.children[index].insertAdjacentElement("beforebegin", event.target);
}

function toggleTagPopup() {
    document.getElementById("tag-search").value = "";
    document.getElementById("tags-popup").classList.toggle("hidden");
}

function compareNames(name1, name2) {
    return name1.localeCompare(name2, undefined, { "caseFirst": "upper" });
}

function closeAutocomplete() {
    let acItems = document.getElementsByClassName("user-ac-container");
    for (let item of acItems) {
        item.remove();
    }
}

function autocomplete(event, usernames) {
    let value = event.target.value;

    closeAutocomplete();

    if (!value) {
        return false;
    }

    acSelected = -1;

    let resultContainer = document.createElement("div");
    resultContainer.setAttribute("id", event.target.id + "-ac-list");
    resultContainer.classList.add("user-ac-container");

    event.target.parentNode.appendChild(resultContainer);

    let ind = binarySearchIndex(value, usernames, 0, usernames.length - 1, compareNames);

    for (let i = ind; i < ind + MAX_RESULTS; i++) {
        let shared_substr = usernames[i].substr(0, value.length);

        // Do not overflow into users not starting with shared_substr
        if (shared_substr.toLowerCase() != value.toLowerCase()) {
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

function unbanId(pid) {
    return (event) => {
        event.target.parentNode.remove();
        bans = bans.filter((ban) => {
            console.log(ban, pid);
            return ban.pid != pid;
        });

        localStorage.setItem("bans", JSON.stringify(bans));
    }
}

function toggleSettings() {
    let settingsContainer = document.getElementById("settings-container");
    if (!settingsContainer.classList.contains("hidden")) {
        document.getElementById("unban-container").innerHTML = ""

        let newRecCount = document.getElementById("rec-count").value;
        if (newRecCount) {
            try {
                recsPerPage = Number.parseInt(newRecCount);

                if (recsPerPage < 1) {
                    recsPerPage = 30;
                }
                else if (recsPerPage > 500) {
                    recsPerPage = 500;
                }

                localStorage.setItem("recsPerPage", recsPerPage.toString());
            }
            catch {
                console.log("This is NOT working");
            }
        }

        settingsContainer.classList.add("hidden");
        return;
    }

    settingsContainer.classList.remove("hidden");

    let unbanHolder = document.getElementById("unban-container");
    for (const ban of bans.toReversed()) {
        const recHolder = document.createElement("div");
        const recText = document.createElement("span");
        const recBan = document.createElement("button");

        recText.innerHTML = `${ban.name}`;

        recBan.addEventListener("click", unbanId(ban.pid));
        recBan.classList.add("ban-button");

        recHolder.appendChild(recBan);
        recHolder.appendChild(recText);
        recHolder.classList.add("rec");

        unbanHolder.appendChild(recHolder);
    }

    document.getElementById("rec-count").value = recsPerPage;
}

async function setUpPage() {
    const usernamesRes = await fetch("files/usernames.json");
    const usernames = await usernamesRes.json();
    const tagsRes = await fetch("files/tags.json");
    const tags = await tagsRes.json();

    // Put a placeholder value in if there are no existing bans
    if (!localStorage.getItem("bans")) {
        localStorage.setItem("bans", "[]");
    }

    bans = JSON.parse(localStorage.getItem("bans"));

    if (!localStorage.getItem("recsPerPage")) {
        localStorage.setItem("recsPerPage", "30");
    }

    recsPerPage = Number.parseInt(localStorage.getItem("recsPerPage"));

    document.getElementById("search-button").addEventListener('click', () => {
        showRecs(tags);
    });

    for (const tag of tagContainer.children) {
        tag.addEventListener('click', toggleTag);
    }

    document.getElementById("tag-select-button").addEventListener('click', toggleTagPopup);

    usernames.sort(compareNames);

    userSearchElement.addEventListener("input", (e) => autocomplete(e, usernames));
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
        input = input
            .split('')
            // Sanitize
            .map((a) => {
                return a.replace(/[|\\{}()[\]^$+*?.]/g, '\\$&')
            })
            .join('(.?)')
        console.log(input);
        let regex = new RegExp(`^(.*)${input}(.*)$`);
        for (const tag of tagContainer.children) {
            if (!regex.test(tag.innerHTML)) {
                tag.classList.add("indisplay");
            }
            else {
                tag.classList.remove("indisplay");
            }
        }
    });

    document.getElementById("settings-close").addEventListener("click", toggleSettings);
}

window.onload = setUpPage();
