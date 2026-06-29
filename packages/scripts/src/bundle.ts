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
});

function initGlobalNavigation(): void {
    const navigation =
        document.querySelector<HTMLDivElement>(".global-navigation");
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

    const progress = button.querySelector<SVGCircleElement>(
        "[data-scroll-progress]"
    );
    const circumference = 2 * Math.PI * 20;

    const update = (): void => {
        const scrollable =
            document.documentElement.scrollHeight - window.innerHeight;
        const ratio =
            scrollable > 0
                ? Math.min(Math.max(window.scrollY / scrollable, 0), 1)
                : 0;

        if (progress) {
            progress.style.strokeDashoffset = String(
                circumference * (1 - ratio)
            );
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
