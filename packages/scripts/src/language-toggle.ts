// Global navigation language toggle. Switches the current page to another
// language by rewriting the URL's language prefix (the default language lives at
// the root, others under `/<lang>/`), then navigating. A full navigation (not
// SPA) is used so the switch is robust across every page type.

function switchedPath(target: string, defaultLang: string, langs: string[]): string {
    let path = window.location.pathname;

    // Strip an existing non-default language prefix, if present.
    for (const lang of langs) {
        if (lang === defaultLang) {
            continue;
        }
        if (path === `/${lang}` || path.startsWith(`/${lang}/`)) {
            path = path.slice(lang.length + 1) || "/";
            break;
        }
    }

    // Prepend the target language prefix (default language stays at the root).
    if (target !== defaultLang) {
        path = `/${target}${path}`;
    }

    return path + window.location.search + window.location.hash;
}

// The header persists across SPA navigation, so its baked-in current-language
// flag goes stale after an in-place language switch. Re-sync it to the live
// `<html lang>`.
function syncCurrent(container: HTMLElement, defaultLang: string): void {
    const current = document.documentElement.lang || defaultLang;
    container.querySelectorAll<HTMLElement>("[data-lang-switch]").forEach((button) => {
        const isCurrent = button.dataset.langSwitch === current;
        button.classList.toggle("language-toggle__option--current", isCurrent);
        if (isCurrent) {
            button.setAttribute("aria-current", "true");
        } else {
            button.removeAttribute("aria-current");
        }
    });
}

export function initLanguageToggle(): void {
    const container = document.querySelector<HTMLElement>("[data-language-toggle]");
    if (!container) {
        return;
    }

    const defaultLang = container.dataset.defaultLang || "ko";
    const langs = [...container.querySelectorAll<HTMLElement>("[data-lang-switch]")].map(
        (el) => el.dataset.langSwitch || "",
    );

    container.addEventListener("click", (event) => {
        const button = (event.target as Element).closest<HTMLElement>("[data-lang-switch]");
        const target = button?.dataset.langSwitch;
        if (!target || target === (document.documentElement.lang || defaultLang)) {
            return;
        }
        window.location.assign(switchedPath(target, defaultLang, langs));
    });

    syncCurrent(container, defaultLang);
    document.addEventListener("spa:navigate", () => syncCurrent(container, defaultLang));
}
