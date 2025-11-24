// Browser bundle entry point
// This file is bundled and minified for browser usage
// Include it in your HTML: <script type="module" src="/js/bundle.js"></script>

import { debounce, throttle } from "./utils";

// Initialize on DOM ready
document.addEventListener("DOMContentLoaded", () => {
  initScrollToTop();
  initLazyImages();
  initCodeCopy();
});

/**
 * Scroll to top button functionality
 */
function initScrollToTop(): void {
  const button = document.querySelector<HTMLButtonElement>(".scroll-to-top");
  if (!button) return;

  const toggleVisibility = throttle(() => {
    button.classList.toggle("visible", window.scrollY > 300);
  }, 100);

  window.addEventListener("scroll", toggleVisibility);
  button.addEventListener("click", () => {
    window.scrollTo({ top: 0, behavior: "smooth" });
  });
}

/**
 * Lazy load images with IntersectionObserver
 */
function initLazyImages(): void {
  const images = document.querySelectorAll<HTMLImageElement>("img[data-src]");
  if (images.length === 0) return;

  const observer = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          const img = entry.target as HTMLImageElement;
          const src = img.dataset.src;
          if (src) {
            img.src = src;
            img.removeAttribute("data-src");
          }
          observer.unobserve(img);
        }
      });
    },
    { rootMargin: "50px" }
  );

  images.forEach((img) => observer.observe(img));
}

/**
 * Code block copy button
 */
function initCodeCopy(): void {
  const codeBlocks = document.querySelectorAll("pre > code");
  codeBlocks.forEach((block) => {
    const pre = block.parentElement;
    if (!pre) return;

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

// Export for module usage
export { debounce, throttle };
