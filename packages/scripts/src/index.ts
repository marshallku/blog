function toggleTheme() {
    const themes = ["light", "sepia", "dark"] as const;

    const currentTheme = document.documentElement.dataset
        .theme as (typeof themes)[number];
    const nextTheme =
        themes[(themes.indexOf(currentTheme) + 1) % themes.length];

    document.documentElement.dataset.theme = nextTheme;
    document.documentElement.style.colorScheme =
        nextTheme === "dark" ? nextTheme : "light";

    const userPrefersTheme = window.matchMedia("(prefers-color-scheme: dark)")
        .matches
        ? "dark"
        : "light";
    if (userPrefersTheme !== nextTheme) {
        localStorage.setItem("theme", nextTheme);
    } else {
        localStorage.removeItem("theme");
    }
}

function main() {
    document.querySelectorAll(".theme-toggle").forEach((element) => {
        element.addEventListener("click", toggleTheme);
    });
}

main();
