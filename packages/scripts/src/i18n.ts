// Runtime UI strings for browser scripts, selected by the page language
// (`<html lang>`). This is a small, self-contained mirror of the handful of
// user-facing strings the scripts render at runtime; the bulk of UI text is
// localized server-side via the SSG's i18n/ui.yaml catalog.

type Lang = "ko" | "en";

const STRINGS = {
    anonymous: { ko: "익명", en: "Anonymous" },
    commentsLoadError: { ko: "댓글을 불러오지 못했습니다.", en: "Failed to load comments." },
    commentPostError: { ko: "댓글 등록에 실패했습니다.", en: "Failed to post comment." },
    copied: { ko: "복사됨!", en: "Copied!" },
    error: { ko: "오류", en: "Error" },
} satisfies Record<string, Record<Lang, string>>;

function currentLang(): Lang {
    return document.documentElement.lang === "en" ? "en" : "ko";
}

export function t(key: keyof typeof STRINGS): string {
    return STRINGS[key][currentLang()];
}
