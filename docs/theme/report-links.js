(() => {
    const reports = [
        ["Lint catalog", "https://sagan-software.github.io/dylints/"],
        ["Coverage report", "https://sagan-software.github.io/dylints/coverage/"],
        ["Benchmark report", "https://sagan-software.github.io/dylints/benches/report/"],
    ];

    function addReportLinks() {
        const chapter = document.querySelector("#sidebar .chapter");
        if (!chapter || document.querySelector("#site-reports")) {
            return;
        }

        const heading = document.createElement("li");
        heading.className = "part-title";
        heading.id = "site-reports";
        heading.textContent = "Reports";
        chapter.append(heading);

        for (const [label, href] of reports) {
            const item = document.createElement("li");
            item.className = "chapter-item expanded";

            const link = document.createElement("a");
            link.href = href;
            link.textContent = label;
            item.append(link);
            chapter.append(item);
        }
    }

    if (document.readyState === "loading") {
        document.addEventListener("DOMContentLoaded", addReportLinks, { once: true });
    } else {
        addReportLinks();
    }
})();
