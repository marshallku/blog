import { initSpa } from "./spa";
import { initToc } from "./toc";
import { initShare } from "./share";
import { initViewCounter } from "./view";
import { fit } from "./utils";

document.addEventListener("DOMContentLoaded", () => {
    initGlobalNavigation();
    initScrollToTop();
    initCodeCopy();
    initSpa();
    initToc();
    initShare();
    initViewCounter();
    initStatusBarClock();
    initHeroTyping();
});

document.addEventListener("spa:navigate", () => {
    initHeroTyping();
});

function initGlobalNavigation(): void {
    const navigation = document.querySelector<HTMLDivElement>(".global-navigation");
    if (!navigation) {
        return;
    }

    const handleScroll = fit(() => {
        if (window.scrollY >= 10) {
            navigation.classList.add("global-navigation--scrolled");
        } else {
            navigation.classList.remove("global-navigation--scrolled");
        }
    });

    window.addEventListener("scroll", handleScroll, { passive: true });
}

function initScrollToTop(): void {
    const button = document.querySelector<HTMLButtonElement>("[data-scroll-top]");
    if (!button) {
        return;
    }

    // Progress is a rounded <rect>; its perimeter is read from the geometry
    // (getTotalLength) so the dash math stays correct regardless of size/radius.
    const progress = button.querySelector<SVGGeometryElement>("[data-scroll-progress]");
    let perimeter = 0;

    const update = (): void => {
        if (progress && perimeter === 0) {
            perimeter = progress.getTotalLength();
            progress.style.strokeDasharray = String(perimeter);
        }

        const scrollable = document.documentElement.scrollHeight - window.innerHeight;
        const ratio = scrollable > 0 ? Math.min(Math.max(window.scrollY / scrollable, 0), 1) : 0;

        if (progress) {
            progress.style.strokeDashoffset = String(perimeter * (1 - ratio));
        }

        button.classList.toggle("scroll-to-top--visible", window.scrollY > 300);
    };

    const onScroll = fit(update);

    update();
    window.addEventListener("scroll", onScroll, { passive: true });
    window.addEventListener("resize", onScroll, { passive: true });
    button.addEventListener("click", () => {
        window.scrollTo({ top: 0, behavior: "smooth" });
    });
}

let heroTypingTimer = 0;

function initHeroTyping(): void {
    // Clear before the early return so navigating away from home stops an
    // in-flight animation from writing to a now-detached element.
    window.clearTimeout(heroTypingTimer);

    const typed = document.querySelector<HTMLElement>(".hero-terminal__typed");
    if (!typed) {
        return;
    }

    const full = typed.dataset.typed ?? typed.textContent ?? "";

    if (!full || window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
        typed.textContent = full;
        return;
    }

    typed.textContent = "";
    let i = 0;
    const tick = (): void => {
        i += 1;
        typed.textContent = full.slice(0, i);
        if (i < full.length) {
            heroTypingTimer = window.setTimeout(tick, 70);
        }
    };
    heroTypingTimer = window.setTimeout(tick, 200);
}

function initStatusBarClock(): void {
    const clock = document.querySelector<HTMLElement>("[data-clock]");
    if (!clock) {
        return;
    }

    const pad = (n: number): string => String(n).padStart(2, "0");
    const update = (): void => {
        const now = new Date();
        clock.textContent = `${pad(now.getHours())}:${pad(now.getMinutes())}:${pad(now.getSeconds())}`;
    };

    update();
    window.setInterval(update, 1000);
}

function initCodeCopy(): void {
    const codeBlocks = document.querySelectorAll("pre > code");
    codeBlocks.forEach((block) => {
        const pre = block.parentElement;
        if (!pre) {
            return;
        }

        const button = document.createElement("button");
        button.className = "copy-code";
        button.textContent = "Copy";
        button.type = "button";

        button.addEventListener("click", async () => {
            const code = block.textContent || "";
            try {
                await navigator.clipboard.writeText(code);
                button.textContent = "Copied!";
                setTimeout(() => {
                    button.textContent = "Copy";
                }, 2000);
            } catch {
                button.textContent = "Failed";
            }
        });

        pre.style.position = "relative";
        pre.appendChild(button);
    });
}
