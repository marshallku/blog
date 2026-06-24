const ACTIVE_CLASS = "post-toc__link--active";

let observer: IntersectionObserver | null = null;

function setup(): void {
    observer?.disconnect();
    observer = null;

    const links = Array.from(
        document.querySelectorAll<HTMLAnchorElement>("[data-toc-link]")
    );

    if (links.length === 0) {
        return;
    }

    const linkBySlug = new Map<string, HTMLAnchorElement>();
    const headings: HTMLElement[] = [];

    links.forEach((link) => {
        const slug = link.getAttribute("data-toc-link");
        if (!slug) {
            return;
        }

        linkBySlug.set(slug, link);

        const heading = document.getElementById(slug);
        if (heading) {
            headings.push(heading);
        }
    });

    if (headings.length === 0) {
        return;
    }

    observer = new IntersectionObserver(
        (entries) => {
            entries.forEach((entry) => {
                if (!entry.isIntersecting) {
                    return;
                }

                links.forEach((link) => link.classList.remove(ACTIVE_CLASS));
                linkBySlug.get(entry.target.id)?.classList.add(ACTIVE_CLASS);
            });
        },
        { rootMargin: "0px 0px -70% 0px", threshold: 0 }
    );

    headings.forEach((heading) => observer?.observe(heading));
}

export function initToc(): void {
    setup();
    document.addEventListener("spa:navigate", setup);
}
