import { createElement } from "react";
import { createRoot, type Root } from "react-dom/client";

const ISLAND_SELECTOR = ".react-island";
const HYDRATED_CLASS = "react-island--hydrated";

type ComponentLoader = () => Promise<{ default: React.ComponentType<unknown> }>;

const componentRegistry: Record<string, ComponentLoader> = {
    Chart: () => import("./components/Chart"),
};

const hydratedRoots = new WeakMap<HTMLElement, Root>();

interface IslandElement extends HTMLElement {
    dataset: DOMStringMap & {
        component: string;
        props: string;
        loading: string;
    };
}

async function hydrateIsland(element: IslandElement): Promise<void> {
    const componentName = element.dataset.component;
    const propsString = element.dataset.props || "{}";

    if (element.classList.contains(HYDRATED_CLASS)) {
        return;
    }

    const loader = componentRegistry[componentName];
    if (!loader) {
        console.error(`React Island: Unknown component "${componentName}"`);
        return;
    }

    try {
        const props = JSON.parse(propsString);
        const { default: Component } = await loader();

        const fallback = element.querySelector(".react-island__fallback");
        if (fallback) {
            fallback.remove();
        }

        const root = createRoot(element);
        root.render(createElement(Component, props));

        hydratedRoots.set(element, root);
        element.classList.add(HYDRATED_CLASS);
    } catch (error) {
        console.error(`React Island: Failed to hydrate "${componentName}"`, error);
    }
}

function cleanupIsland(element: HTMLElement): void {
    const root = hydratedRoots.get(element);
    if (root) {
        root.unmount();
        hydratedRoots.delete(element);
        element.classList.remove(HYDRATED_CLASS);
    }
}

export function initIslands(container: Element | Document = document): void {
    const islands = container.querySelectorAll<IslandElement>(ISLAND_SELECTOR);

    islands.forEach((island) => {
        if (island.classList.contains(HYDRATED_CLASS)) {
            return;
        }

        const loading = island.dataset.loading || "lazy";

        if (loading === "eager") {
            hydrateIsland(island);
        } else {
            const observer = new IntersectionObserver(
                (entries, obs) => {
                    entries.forEach((entry) => {
                        if (entry.isIntersecting) {
                            hydrateIsland(entry.target as IslandElement);
                            obs.unobserve(entry.target);
                        }
                    });
                },
                { rootMargin: "50px" }
            );
            observer.observe(island);
        }
    });
}

export function cleanupIslands(container: Element | Document = document): void {
    const islands = container.querySelectorAll<HTMLElement>(
        `${ISLAND_SELECTOR}.${HYDRATED_CLASS}`
    );
    islands.forEach(cleanupIsland);
}

export function reInitIslands(container: Element): void {
    cleanupIslands(container);
    initIslands(container);
}

export function registerComponent(name: string, loader: ComponentLoader): void {
    componentRegistry[name] = loader;
}

if (typeof document !== "undefined") {
    document.addEventListener("DOMContentLoaded", () => {
        initIslands();
    });

    document.addEventListener("spa:navigate", (event) => {
        const container = (event as CustomEvent).detail?.container;
        if (container instanceof Element) {
            reInitIslands(container);
        } else {
            initIslands();
        }
    });
}
