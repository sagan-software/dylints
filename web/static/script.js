/*
 * Adapted from rust-clippy util/gh-pages/script.js at commit
 * 9066dbb89ca01eddd591d03a2b4504ab28d7c26b. Clippy is licensed under
 * MIT OR Apache-2.0; see LICENSE-CLIPPY-MIT.
 */
"use strict";

const searchState = {
    lastSearch: '',
    filterLints: (updateURL = true) => {
        function matchesSearch(lint, terms, searchStr) {
            // Search by id
            if (lint.elem.id.indexOf(searchStr) !== -1) {
                return true;
            }
            // Search the description
            // The use of `for`-loops instead of `foreach` enables us to return early
            const docsLowerCase = lint.elem.textContent.toLowerCase();
            for (const term of terms) {
                // This is more likely and will therefore be checked first
                if (docsLowerCase.indexOf(term) !== -1) {
                    return true;
                }

                if (lint.elem.id.indexOf(term) !== -1) {
                    return true;
                }

                return false;
            }
            return true;
        }

        const searchStr = elements.search.value.trim().toLowerCase();
        if (searchState.lastSearch === searchStr) {
            return;
        }
        searchState.lastSearch = searchStr;
        const terms = searchStr.split(" ");
        const cleanedSearchStr = searchStr.replaceAll("-", "_");

        for (const lint of filters.getAllLints()) {
            lint.searchFilteredOut = !matchesSearch(lint, terms, cleanedSearchStr);
            if (lint.searchFilteredOut || lint.filteredOut) {
                lint.elem.style.display = "none";
            } else {
                lint.elem.style.display = "";
            }
        }

        if (updateURL) {
            setURL();
        }
    },
};

function handleInputChanged(event) {
    if (event.target !== document.activeElement) {
        return;
    }
    searchState.filterLints();
}

function handleShortcut(ev) {
    if (ev.ctrlKey || ev.altKey || ev.metaKey || disableShortcuts) {
        return;
    }

    if (document.activeElement.tagName === "INPUT") {
        if (ev.key === "Escape") {
            document.activeElement.blur();
        }
    } else {
        switch (ev.key) {
            case "s":
            case "S":
            case "/":
                ev.preventDefault(); // To prevent the key to be put into the input.
                elements.search.focus();
                break;
            default:
                break;
        }
    }
}

/**
 * When `value` is `null` the default value is used - true for all but the `deprecated` lint group
 */
function resetCheckboxes(filter, value) {
    for (const element of elements.checkboxes[filter]) {
        const checked = value ?? element.name !== "deprecated";
        element.checked = checked;
    }
}

function onEachLazy(lazyArray, func) {
    const arr = Array.prototype.slice.call(lazyArray);
    for (const el of arr) {
        func(el);
    }
}

function expandLint(lintId) {
    const elem = document.querySelector(`#${lintId} > input[type="checkbox"]`);
    if (elem) {
        elem.checked = true;
    }
}

const clipboardTimeouts = new Map();
function copyToClipboard(event) {
    event.preventDefault();
    event.stopPropagation();

    const clipboard = event.target;

    navigator.clipboard.writeText(clipboard.parentElement.id.slice(5));

    clipboard.textContent = "✓";

    clearTimeout(clipboardTimeouts.get(clipboard));
    clipboardTimeouts.set(
        clipboard,
        setTimeout(() => {
            clipboard.textContent = "📋";
            clipboardTimeouts.delete(clipboard);
        }, 1000)
    );
}

function toggleExpansion(expand) {
    for (const checkbox of document.querySelectorAll("article input[type=checkbox]")) {
        checkbox.checked = expand;
    }
}

const filters = {
    allLints: null,
    getAllLints: () => {
        if (filters.allLints === null) {
            filters.allLints = Array.prototype.slice.call(
                document.getElementsByTagName("article"),
            ).map(elem => {
                return {
                    elem: elem,
                    group: elem.querySelector(".lint-group").innerText,
                    level: elem.querySelector(".lint-level").innerText,
                    applicability: elem.querySelector(".applicability").innerText,
                    filteredOut: false,
                    searchFilteredOut: false,
                };
            });
        }
        return filters.allLints;
    },
    filterLints: (updateURL = true) => {
        const [levels, groups, applicabilities] = ["levels", "groups", "applicabilities"].map(key => new Set(
            elements.checkboxes[key]
                .filter(checkbox => checkbox.checked)
                .map(checkbox => checkbox.name)
        ));

        elements.counts.groups.textContent = groups.size;
        elements.counts.levels.textContent = levels.size;
        elements.counts.applicabilities.textContent = applicabilities.size;

        for (const lint of filters.getAllLints()) {
            lint.filteredOut = (!groups.has(lint.group)
                || !levels.has(lint.level)
                || !applicabilities.has(lint.applicability)
            );
            if (lint.filteredOut || lint.searchFilteredOut) {
                lint.elem.style.display = "none";
            } else {
                lint.elem.style.display = "";
            }
        }

        if (updateURL) {
            setURL();
        }
    },
};

function setupDropdown(elem) {
    const button = elem.querySelector("button");
    button.onclick = () => elem.classList.toggle("open");

    const setBlur = child => {
        child.onblur = event => {
            if (!elem.contains(document.activeElement) &&
                !elem.contains(event.relatedTarget)
            ) {
                elem.classList.remove("open");
            }
        }
    };
    onEachLazy(elem.children, setBlur);
    onEachLazy(elem.querySelectorAll("select"), setBlur);
    onEachLazy(elem.querySelectorAll("input"), setBlur);
    onEachLazy(elem.querySelectorAll("ul button"), setBlur);
}

function setURL() {
    const url = new URL(location);

    function nonDefault(filter) {
        return elements.checkboxes[filter]
            .some(element => element.checked === (element.name === "deprecated"));
    }
    function setBoolean(filter) {
        if (nonDefault(filter)) {
            const value = elements.checkboxes[filter]
                .filter(el => el.checked)
                .map(el => el.name)
                .join(",");
            url.searchParams.set(filter, value);
        }
    }

    url.search = "";
    setBoolean("groups");
    setBoolean("levels");
    setBoolean("applicabilities");

    const search = elements.search.value;
    if (search) {
        url.searchParams.set("search", search)
    }

    url.hash = "";

    if (!history.state) {
        history.pushState(true, "", url);
    } else {
        history.replaceState(true, "", url);
    }
}

function parseURL() {
    const params = new URLSearchParams(window.location.search);

    for (const [filter, checkboxes] of Object.entries(elements.checkboxes)) {
        if (params.has(filter)) {
            const settings = new Set(params.get(filter).split(","));

            for (const checkbox of checkboxes) {
                checkbox.checked = settings.has(checkbox.name);
            }
        } else {
            resetCheckboxes(filter, null);
        }
    }

    elements.search.value = params.get("search");

    if (location.hash) {
        expandLint(location.hash.slice(1));
    }

    filters.filterLints(false);
    searchState.filterLints(false);
}

function addResetListeners(selector, value) {
    for (const button of document.querySelectorAll(selector)) {
        button.addEventListener("click", event => {
            const container = event.target.closest("[data-filter]");
            const filter = container.dataset.filter;
            resetCheckboxes(filter, value);
            filters.filterLints();
        })
    }
}

function addListeners() {
    document.getElementById("upper-filters").addEventListener("input", () => {
        filters.filterLints();
    });
    elements.search.addEventListener("input", handleInputChanged);

    elements.disableShortcuts.addEventListener("change", () => {
        disableShortcuts = elements.disableShortcuts.checked;
        storeValue("disable-shortcuts", disableShortcuts);
    });

    document.getElementById("expand-all").addEventListener("click", () => toggleExpansion(true));
    document.getElementById("collapse-all").addEventListener("click", () => toggleExpansion(false));

    // A delegated listener to avoid the upfront cost of >1000 listeners
    document.addEventListener("click", event => {
        if (!event.target instanceof HTMLAnchorElement) {
            return;
        }

        if (event.target.classList.contains("copy-to-clipboard")) {
            copyToClipboard(event);
        } else if (event.target.classList.contains("anchor")) {
            event.target.closest("article")
                .querySelector(`input[type="checkbox"]`)
                .checked = true;
        }
    });

    document.getElementById("filter-clear").addEventListener("click", () => {
        elements.search.value = "";
        searchState.filterLints();
    })

    addResetListeners(".reset-all", true);
    addResetListeners(".reset-none", false);
    addResetListeners(".reset-default", null);

    document.addEventListener("keypress", handleShortcut);
    document.addEventListener("keydown", handleShortcut);

    document.querySelectorAll(".dropdown").forEach(setupDropdown);

    addEventListener("popstate", parseURL);
}

// Highlight code blocks only when they approach the viewport so that clicking the "Expand All"
// button doesn't take a long time
function highlightLazily() {
    if (!'IntersectionObserver' in window) {
        return;
    }
    const observer = new IntersectionObserver((entries) => {
        for (const entry of entries) {
            if (entry.isIntersecting) {
                observer.unobserve(entry.target);
                for (const code of entry.target.querySelectorAll("pre code")) {
                    hljs.highlightElement(code);
                }
            }
        }
    });
    for (const docs of document.querySelectorAll(".lint-docs")) {
        observer.observe(docs);
    }
}

function findCheckboxes(filter) {
    return [...document.querySelectorAll(`.dropdown[data-filter="${filter}"] input[type="checkbox"]`)];
}

let disableShortcuts = loadValue("disable-shortcuts") === "true";

const elements = {
    search: document.getElementById("search-input"),
    disableShortcuts: document.getElementById("disable-shortcuts"),
    checkboxes: {
        levels: findCheckboxes("levels"),
        groups: findCheckboxes("groups"),
        applicabilities: findCheckboxes("applicabilities"),
    },
    counts: {
        levels: document.getElementById("levels-count"),
        groups: document.getElementById("groups-count"),
        applicabilities: document.getElementById("applicabilities-count"),
    },
};

elements.disableShortcuts.checked = disableShortcuts;

addListeners();
highlightLazily();
parseURL();
