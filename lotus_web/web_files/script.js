// Locally stored array of the pids of banned pages
let bans = undefined;
let tagStrings = [];

// Setting up local storage
if (typeof (Storage) !== "undefined") {
    // Put a placeholder value in if there are no existing bans
    if (!localStorage.getItem("bans")) {
        window.localStorage.setItem("bans", "[]");
    }

    bans = JSON.parse(window.localStorage.getItem("bans"));
    console.log("bans: ", bans);
} else {
    console.log("No storage support...");
}

let wikiPrefix = "https://scp-wiki.wikidot.com/";
// Retrieves and displays recommendations for the given user with the given settings
async function showRecs() {
    console.log("Starting to request...");

    console.log(document.getElementById("user-search").value);

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

    // TODO add a bunch of error-checking/pruning here
    if (bans.length > 0) {
        url += "&bans=";
        for (const ban of bans) {
            url += ban;
            url += "+"
        }
        url = url.substring(0, url.length - 1);
    }

    console.log(url);

    try {
        // let json = [{ "name": "Integrity Project", "url": "integrity-project", "tags": ["_licensebox", "director-aktus", "resurrection", "tale"] }, { "name": "Like We Were Ever Kindergarten Teachers to Start With", "url": "maria-jones-this-is-your-life", "tags": ["_licensebox", "doctor-bright", "doctor-light", "maria-jones", "resurrection", "tale"] }, { "name": "All This Wandering", "url": "but-some-time-we-cant-erase", "tags": ["_adult", "_licensebox", "maria-jones", "resurrection", "tale"] }, { "name": "I Was Not Magnificent", "url": "i-was-not-magnificent", "tags": ["_licensebox", "director-gillespie", "doctor-light", "doctor-roget", "resurrection", "rewritable", "tale"] }, { "name": "SCP-173", "url": "scp-173", "tags": ["_licensebox", "autonomous", "ectoentropic", "euclid", "featured", "hostile", "observational", "scp", "sculpture", "the-sculpture"] }, { "name": "Where Your Eyes Don't Go", "url": "where-your-eyes-don-t-go", "tags": ["_licensebox", "director-gillespie", "doctor-roget", "doctor-vang", "researcher-conwell", "researcher-rosen", "resurrection", "rewritable", "tale"] }, { "name": "wowwee go kill ursefl", "url": "wowwee-go-kill-ursefl", "tags": ["_licensebox", "are-we-cool-yet", "black-comedy", "comedy", "ruiz-duchamp", "tale"] }, { "name": "The Cool War", "url": "the-cool-war-hub", "tags": ["_licensebox", "_tale-hub", "are-we-cool-yet", "dr-wondertainment", "hub", "nobody"] }, { "name": "SCP-6001", "url": "scp-6001", "tags": ["6000", "_cc", "_licensebox", "anderson", "animal", "are-we-cool-yet", "children-of-the-night", "esoteric-class", "extradimensional", "feline", "global-occult-coalition", "hard-to-destroy-reptile", "manna-charitable-foundation", "marshall-carter-and-dark", "nameless", "nobody", "parawatch", "portal", "prometheus", "sapient", "scp", "serpents-hand", "the-sculpture", "wilsons-wildlife"] }, { "name": "SCP-082", "url": "scp-082", "tags": ["_licensebox", "alive", "euclid", "humanoid", "predatory", "sapient", "scp"] }, { "name": "The Cool Kids", "url": "the-cool-kids", "tags": ["_licensebox", "are-we-cool-yet", "comedy", "spy-fiction", "tale", "the-critic"] }, { "name": "Friendly Conversation", "url": "friendly-conversation", "tags": ["_licensebox", "bittersweet", "iris-thompson", "reviewers-spotlight", "slice-of-life", "tale"] }, { "name": "Kill the Feeling", "url": "kill-the-feeling", "tags": ["_licensebox", "bleak", "coldpostcon", "iris-thompson", "lgbtq", "tale"] }, { "name": "Moonlighting", "url": "moonlighting", "tags": ["_licensebox", "are-we-cool-yet", "eurtec", "global-occult-coalition", "prometheus", "silicon-nornir", "tale", "third-law"] }, { "name": "The Department of Humanoid Risk Assessment", "url": "humanoid-risk-assessment", "tags": ["_licensebox", "featured", "iris-thompson", "tale", "worldbuilding"] }, { "name": "Clef And Dimitri Hit The Road", "url": "clef-and-dimitri-hit-the-road", "tags": ["_licensebox", "agent-strelnikov", "agent-yoric", "co-authored", "comedy", "doctor-clef", "doctor-glass", "global-occult-coalition", "tale"] }, { "name": "Routine Psychological Evaluations By Dr Glass", "url": "routine-psychological-evaluations-by-dr-glass", "tags": ["_licensebox", "comedy", "doctor-bright", "doctor-clef", "doctor-gears", "doctor-glass", "doctor-kondraki", "doctor-rights", "slice-of-life", "tale"] }, { "name": "SCP-1000", "url": "scp-1000", "tags": ["1000", "_cc", "_licensebox", "alive", "children-of-the-night", "featured", "historical", "humanoid", "illustrated", "k-class-scenario", "keter", "sapient", "scp", "serpents-hand", "species", "uncontained"] }, { "name": "Penal Reform", "url": "penal-reform", "tags": ["_licensebox", "iris-thompson", "rainer-miller", "reviewers-spotlight", "slice-of-life", "tale"] }, { "name": "Onboarding", "url": "onboarding", "tags": ["_licensebox", "comedy", "iris-thompson", "rainer-miller", "slice-of-life", "tale"] }, { "name": "SCP-085", "url": "scp-085", "tags": ["_cc", "_licensebox", "artistic", "autonomous", "foundation-made", "humanoid", "inscription", "mobile", "safe", "sapient", "scp", "visual"] }, { "name": "Personal Log of Agent AA", "url": "log-of-agent-aa", "tags": ["_licensebox", "able", "cain", "first-person", "iris-thompson", "journal", "tale"] }, { "name": "Ouroboros", "url": "ouroboros", "tags": ["001-proposal", "_cc", "_licensebox", "hub", "splash"] }, { "name": "SCP-2343", "url": "scp-2343", "tags": ["_cc", "_licensebox", "_listpages", "biological", "humanoid", "keter", "meta", "prize-feature", "reality-bending", "sapient", "scp"] }, { "name": "Voices Carry: Part 1", "url": "voices-carry-part-1", "tags": ["_licensebox", "action", "featured", "global-occult-coalition", "iris-thompson", "last-hope", "military-fiction", "resurrection", "tale"] }, { "name": "We Need To Talk About Fifty-Five", "url": "we-need-to-talk-about-fifty-five", "tags": ["_licensebox", "marion-wheeler", "tale"] }, { "name": "Mobile Task Force Basic School: Induction Remarks", "url": "sunday-0600-mobile-task-force-central-training-facility", "tags": ["_licensebox", "angle-grinders", "lombardi", "military-fiction", "orientation", "tale"] }, { "name": "black white black white black white black white black white gray", "url": "black-white-black-white-black-white-black-white-black-white", "tags": ["_licensebox", "tale"] }, { "name": "SCP-4818", "url": "scp-4818", "tags": ["_licensebox", "euclid", "humanoid", "last-hope", "light", "sapient", "scp", "superhero"] }, { "name": "Excerpts From \"How To Survive When Reality Doesn't\", by Alto Clef", "url": "clef-excerpts", "tags": ["_licensebox", "cain", "doctor-clef", "tale"] }]

        // TODO add loading screen here

        const response = await fetch(url);
        if (!response.ok) {
            throw new Error(`Response status: ${response.status}`);
        }

        json = await response.json();
        console.log(json);

        document.getElementById("rec-container-container").classList.remove("hidden");

        let recommendationsHolder = document.getElementById("rec-container");

        // Clear of previous recs
        recommendationsHolder.innerHTML = ""

        // Add starting dummy
        recommendationsHolder.appendChild(document.createElement("div"));

        function banId(id) {
            console.log(`Banning: ${id}`);
            return function ban(event) {
                event.target.parentNode.remove();
                bans.push(id);
                window.localStorage.setItem("bans", JSON.stringify(bans));
            }
        }

        // TODO add error handling
        for (const page of json.slice(0, 30)) {
            const recHolder = document.createElement("div");
            const recLink = document.createElement("a");
            const recBan = document.createElement("button");

            recLink.innerHTML = `${page.name}`;

            recLink.setAttribute("href", wikiPrefix + page.url);
            recLink.setAttribute("target", "_blank");
            recLink.classList.add("rec-link");

            recBan.addEventListener("click", banId(page.pid));
            recBan.classList.add("ban-button");

            recHolder.appendChild(recBan);
            recHolder.appendChild(recLink);
            recHolder.classList.add("rec");

            recommendationsHolder.appendChild(recHolder);
        }

        // Add ending dummy
        recommendationsHolder.appendChild(document.createElement("div"));

    } catch (error) {
        console.error(error.message);
    }

    console.log("Done!");
}

document.getElementById("search-button").addEventListener('click', showRecs);

// Add everything to the tags element
// CONS actually do this with templeting in the final
let tagContainer = document.getElementById("tag-container");
for (const tag of tags) {
    let newTag = document.createElement("span");
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
    let mid = 0;

    // If this WAS selected, it shouldn't be anymore
    if (selected) {
        low = tagStrings.length;
        high = tagContainer.children.length - 1;
    }
    else {
        low = 0;
        high = tagStrings.length - 2;
    }

    let test;
    while (low <= high) {
        mid = Math.floor((high + low) / 2);

        let text = tagContainer.children[mid].innerHTML;
        test = text.toString().replace(/^_+/, "") < event.target.innerHTML.replace(/^_+/, "");

        if (text == event.target.innerHTML) {
            console.log("ERROR: tag to place already found");
            return;
        }
        else if (test) {
            low = mid + 1;
        }
        else {
            high = mid - 1;
        }
    }

    if (test) {
        tagContainer.children[mid].insertAdjacentElement("afterend", event.target);
    }
    else {
        tagContainer.children[mid].insertAdjacentElement("beforebegin", event.target);
    }
}

for (const tag of tagContainer.children) {
    tag.addEventListener('click', toggleTag);
}

function toggleTagPopup() {
    document.getElementById("tags-popup").classList.toggle("hidden");
}

document.getElementById("tag-select-button").addEventListener('click', toggleTagPopup);
