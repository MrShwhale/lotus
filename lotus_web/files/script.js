// The number of results to show when autocompleting users
const MAX_RESULTS = 5;

// Prefix of the SCP wiki
const WIKI_PREFIX = "https://scp-wiki.wikidot.com/";

let acSelected = -1;
let bans = undefined;
let isRequesting = false;
let recs = [];
let recsPerPage = 30;
let tagContainer = document.getElementById("tag-container");
let tagStrings = [];
let userSearchElement = document.getElementById("user-search");

// Compares two usernames on the wiki
function compareNames(name1, name2) {
    return name1.localeCompare(name2, undefined, { "caseFirst": "upper" });
}

// Show/hide the tag popup
function toggleTagPopup() {
    document.getElementById("tag-search").value = "";
    document.getElementById("tags-popup").classList.toggle("hidden");
}

// Compares the given tag elements in the same way wikidot does, to keep ordering
function compareTagElements(tag1, tag2) {
    let tag1_stripped = tag1.innerHTML.replace("-", "").replace("_", "");
    let tag2_stripped = tag2.innerHTML.replace("-", "").replace("_", "");
    let test = tag1_stripped.localeCompare(tag2_stripped);

    if (test == 0) {
        return tag1.innerHTML.localeCompare(tag2.innerHTML);
    }

    return test;
}

// Takes a value, array, start/end indices (inclusive), and compare function
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

// Select a tag to add it to the recommendation filters, or unselect a tag to 
// remove it from the filters.
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

    // Take out the element now so the indices are correct
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

// Create a function to add the give pid/name to the ban list
function createIdBanFunc(id, name) {
    return (event) => {
        event.target.parentNode.remove();
        bans.push({ pid: id, name: name });
        window.localStorage.setItem("bans", JSON.stringify(bans));

        recs = recs.filter((value) => {
            return value.pid != id;
        });

        displayRecs(recs);
    }
}

// Display the given recommendations in the recommendations container
// Assumes the recommendations container is not hidden in any way
function displayRecs(recs) {
    let recommendationsContainer = document.getElementById("rec-container");

    recommendationsContainer.innerHTML = "";

    // Add starting dummy for functional flex formatting
    recommendationsContainer.appendChild(document.createElement("div"));

    for (const page of recs.slice(0, recsPerPage)) {
        const recContainer = document.createElement("div");
        const recLink = document.createElement("a");
        const recBan = document.createElement("button");

        recLink.innerHTML = `${page.name}`;

        recLink.setAttribute("href", WIKI_PREFIX + page.url);
        recLink.setAttribute("target", "_blank");
        recLink.classList.add("rec-link");

        recBan.addEventListener("click", createIdBanFunc(page.pid, page.name));
        recBan.classList.add("ban-button");

        recContainer.appendChild(recBan);
        recContainer.appendChild(recLink);
        recContainer.classList.add("rec");

        recommendationsContainer.appendChild(recContainer);
    }

    // Add ending dummy for functional flex formatting
    recommendationsContainer.appendChild(document.createElement("div"));
}

// Retrieves and displays recommendations for the given user with the given settings
async function showRecs(tags) {
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

        // Filter out anything which isn't a proper bans object with a pid that is a number
        let oldLen = bans.length;
        bans = bans.filter((item) => {
            return item.hasOwnProperty('pid') && item.hasOwnProperty('name') && item.pid == Number.parseInt(item.pid);
        });

        if (oldLen != bans.length) {
            window.localStorage.setItem("bans", JSON.stringify(bans));
        }

        for (const ban of bans) {
            url += ban.pid;
            url += "+"
        }
        url = url.substring(0, url.length - 1);
    }

    try {
        document.getElementById("rec-container-container").classList.remove("hidden");

        let recommendationsContainer = document.getElementById("rec-container");

        // Clear of previous recs/errors
        recommendationsContainer.innerHTML = "";

        let loadingElement = document.createElement("div");
        loadingElement.classList.add("response-text");
        loadingElement.innerHTML = "LOADING REQUESTS...";

        recommendationsContainer.appendChild(loadingElement);

        const response = await fetch(url);
        if (!response.ok) {
            console.log(response);

            recommendationsContainer.innerHTML = "";

            let errorElement = document.createElement("div");
            errorElement.classList.add("response-text");
            errorElement.classList.add("error");
            errorElement.innerHTML = `ERROR ${response.status}`;

            let statusText = document.createElement("p");
            statusText.classList.add("status-text");
            statusText.innerHTML = response.statusText;

            recommendationsContainer.appendChild(errorElement);
            recommendationsContainer.appendChild(statusText);

            isRequesting = false;
            throw new Error(`Response status: ${response.status}`);
        }

        isRequesting = false;

        json = await response.json();

        recommendationsContainer.innerHTML = "";

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

            recommendationsContainer.appendChild(errorElement);
            recommendationsContainer.appendChild(statusText);

            throw new Error(`Valid response with error: ${json.code}`);
        }

        recs = json;

        displayRecs(recs);
    } catch (error) {
        console.error(error.message);
    }
}

// Create a function which will remove a page with the given pid from the bans
// list, if it exists
function createIdUnbanFunc(pid) {
    return (event) => {
        event.target.parentNode.remove();
        bans = bans.filter((ban) => {
            return ban.pid != pid;
        });

        window.localStorage.setItem("bans", JSON.stringify(bans));
    }
}

// Hide/show the settings popup. Also saves settings if it is closing.
function toggleSettings() {
    let settingsContainer = document.getElementById("settings-container");
    if (!settingsContainer.classList.contains("hidden")) {
        document.getElementById("unban-container").innerHTML = "";

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

                window.localStorage.setItem("recsPerPage", recsPerPage.toString());
            }
            catch {
                console.log("This is NOT working");
            }
        }

        settingsContainer.classList.add("hidden");
        return;
    }

    settingsContainer.classList.remove("hidden");

    let unbanContainer = document.getElementById("unban-container");
    for (const ban of bans.toReversed()) {
        const recContainer = document.createElement("div");
        const recText = document.createElement("span");
        const recBan = document.createElement("button");

        recText.innerHTML = `${ban.name}`;

        recBan.addEventListener("click", createIdUnbanFunc(ban.pid));
        recBan.classList.add("ban-button");

        recContainer.appendChild(recBan);
        recContainer.appendChild(recText);
        recContainer.classList.add("rec");

        unbanContainer.appendChild(recContainer);
    }

    document.getElementById("rec-count").value = recsPerPage;
}

// Close the user autocomplete popup
function closeAutocomplete() {
    let acItems = document.getElementsByClassName("user-ac-container");
    for (let item of acItems) {
        item.remove();
    }
}

// Set up custom autocomplete for the user-search from the given list of names
function user_autocomplete(usernames) {
    return function(event) {
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
}

// Handle key presses made while user autocomplete is running
function autocompleteHandlePresses(event) {
    let elementList = document.getElementById(event.target.id + "-ac-list");
    if (!elementList) {
        return;
    }
    else {
        elementList = elementList.children;
    }

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

// Run code to get everything in working order after the page has been loaded.
// Loads stored info and adds event listeners
async function setUpPage() {
    const usernameResponse = await fetch("files/usernames.json");
    const usernames = await usernameResponse.json();
    const tagsResponse = await fetch("files/tags.json");
    const tags = await tagsResponse.json();

    usernames.sort(compareNames);

    // Put a placeholder value in if there are no existing bans
    if (!localStorage.getItem("bans")) {
        window.localStorage.setItem("bans", "[]");
    }

    bans = JSON.parse(window.localStorage.getItem("bans"));

    if (!localStorage.getItem("recsPerPage")) {
        window.localStorage.setItem("recsPerPage", "30");
    }

    recsPerPage = Number.parseInt(window.localStorage.getItem("recsPerPage"));

    document.getElementById("tag-select-button").addEventListener('click', toggleTagPopup);

    document.getElementById("tag-search").addEventListener("input", (event) => {
        let input = event.target.value;
        input = input
            .split('')
            // Sanitize user input for Regex use
            .map((a) => {
                return a.replace(/[|\\{}()[\]^$+*?.]/g, '\\$&')
            })
            .join('(.?)')
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

    for (const tag of tagContainer.children) {
        tag.addEventListener('click', toggleTag);
    }

    document.getElementById("settings-close").addEventListener("click", toggleSettings);

    document.getElementById("search-button").addEventListener('click', () => {
        showRecs(tags);
    });

    document.addEventListener("keydown", (event) => {
        if (event.key == "Enter") {
            if (!event.repeat) {
                showRecs(tags)
            }
        }
    });

    userSearchElement.addEventListener("input", user_autocomplete(usernames));
    userSearchElement.addEventListener("keydown", autocompleteHandlePresses);

    document.addEventListener("click", (event) => {
        closeAutocomplete(event.target);
    });
}

window.onload = setUpPage();
