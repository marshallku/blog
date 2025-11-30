type CallBack<T extends any[]> = (...args: T) => any;

const DEFAULT_WAIT = 500;

export function debounce<T extends any[]>(
    fn: CallBack<T>,
    delay: number = DEFAULT_WAIT
): CallBack<T> {
    let timeoutId: ReturnType<typeof setTimeout>;
    return (...args: T) => {
        clearTimeout(timeoutId);
        timeoutId = setTimeout(() => fn(...args), delay);
    };
}

export function throttle<T extends any[]>(
    fn: CallBack<T>,
    delay: number = DEFAULT_WAIT
): CallBack<T> {
    let lastCall = 0;
    return (...args: T) => {
        const now = Date.now();
        if (now - lastCall >= delay) {
            lastCall = now;
            fn(...args);
        }
    };
}

export function fit<T extends any[]>(func: CallBack<T>): () => void {
    let ticking = false;

    return (...args: T) => {
        if (!ticking) {
            ticking = true;
            requestAnimationFrame(() => {
                func(...args);
                ticking = false;
            });
        }
    };
}

export function isInViewport(element: Element): boolean {
    const rect = element.getBoundingClientRect();
    return (
        rect.top >= 0 &&
        rect.left >= 0 &&
        rect.bottom <=
            (window.innerHeight || document.documentElement.clientHeight) &&
        rect.right <=
            (window.innerWidth || document.documentElement.clientWidth)
    );
}
