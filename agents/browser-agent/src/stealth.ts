/**
 * RedNode-OS – Browser Stealth Module
 * Anti-detection for web scraping. Rotates user agents, headers, timing.
 *
 * Techniques:
 *   1. Real browser user-agent rotation (Chrome/Firefox/Safari/Edge, latest versions)
 *   2. Accept-Language and Accept-Encoding randomization
 *   3. Referer chain spoofing (Google/Bing/DuckDuckGo)
 *   4. Request timing jitter (random delays between requests)
 *   5. TLS fingerprint via Playwright (real browser, not curl/fetch)
 *   6. Cookie persistence per domain
 *   7. Viewport randomization
 *   8. WebDriver detection bypass (Playwright stealth patches)
 */

// ─── User Agent Pool ───
// Real, current user agents from actual browsers. Updated for 2026.

const USER_AGENTS = {
  chrome_win: [
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
  ],
  chrome_mac: [
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
  ],
  chrome_linux: [
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
  ],
  firefox_win: [
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:128.0) Gecko/20100101 Firefox/128.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:127.0) Gecko/20100101 Firefox/127.0",
  ],
  firefox_linux: [
    "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0",
    "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0",
  ],
  safari_mac: [
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_5) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.5 Safari/605.1.15",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_4) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Safari/605.1.15",
  ],
  edge_win: [
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36 Edg/126.0.0.0",
  ],
};

const ALL_UAS: string[] = Object.values(USER_AGENTS).flat();

// ─── Accept-Language Pools ───

const ACCEPT_LANGUAGES = [
  "en-US,en;q=0.9",
  "en-GB,en;q=0.9",
  "en-US,en;q=0.9,hi;q=0.8",
  "en-US,en;q=0.9,es;q=0.8",
  "en-US,en;q=0.9,fr;q=0.7",
  "en,en-US;q=0.9",
];

// ─── Referer Pools (search engines) ───

const REFERERS = [
  "https://www.google.com/",
  "https://www.google.com/search?q=",
  "https://www.bing.com/search?q=",
  "https://duckduckgo.com/?q=",
  "https://search.yahoo.com/search?p=",
  "", // direct visit (no referer)
  "", // direct visit
];

// ─── Viewport Sizes ───

const VIEWPORTS = [
  { width: 1920, height: 1080 },
  { width: 1536, height: 864 },
  { width: 1440, height: 900 },
  { width: 1366, height: 768 },
  { width: 1280, height: 720 },
  { width: 2560, height: 1440 },
];

// ─── Randomization Helpers ───

function pick<T>(arr: T[]): T {
  return arr[Math.floor(Math.random() * arr.length)];
}

function randomDelay(minMs: number, maxMs: number): Promise<void> {
  const delay = minMs + Math.random() * (maxMs - minMs);
  return new Promise(resolve => setTimeout(resolve, delay));
}

// ─── Public API ───

export interface StealthProfile {
  userAgent: string;
  acceptLanguage: string;
  referer: string;
  viewport: { width: number; height: number };
  headers: Record<string, string>;
}

/**
 * Generate a randomized stealth profile for a request.
 * Each call produces different but realistic browser fingerprints.
 */
export function generateProfile(targetUrl?: string): StealthProfile {
  const userAgent = pick(ALL_UAS);
  const acceptLanguage = pick(ACCEPT_LANGUAGES);
  const referer = pick(REFERERS);
  const viewport = pick(VIEWPORTS);

  // Determine browser type from UA for consistent Accept header
  const isFirefox = userAgent.includes("Firefox");
  const isSafari = userAgent.includes("Safari") && !userAgent.includes("Chrome");

  const accept = isFirefox
    ? "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8"
    : isSafari
    ? "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"
    : "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8";

  const headers: Record<string, string> = {
    "User-Agent": userAgent,
    "Accept": accept,
    "Accept-Language": acceptLanguage,
    "Accept-Encoding": "gzip, deflate, br",
    "Cache-Control": pick(["no-cache", "max-age=0"]),
    "Upgrade-Insecure-Requests": "1",
    "Sec-Fetch-Dest": "document",
    "Sec-Fetch-Mode": "navigate",
    "Sec-Fetch-Site": referer ? "cross-site" : "none",
    "Sec-Fetch-User": "?1",
  };

  // Add Sec-CH-UA for Chrome/Edge (Client Hints)
  if (userAgent.includes("Chrome") && !isFirefox) {
    const version = userAgent.match(/Chrome\/(\d+)/)?.[1] || "126";
    headers["Sec-CH-UA"] = `"Chromium";v="${version}", "Not/A)Brand";v="8", "Google Chrome";v="${version}"`;
    headers["Sec-CH-UA-Mobile"] = "?0";
    headers["Sec-CH-UA-Platform"] = userAgent.includes("Windows") ? '"Windows"' :
      userAgent.includes("Macintosh") ? '"macOS"' : '"Linux"';
  }

  if (referer) {
    headers["Referer"] = referer + (targetUrl ? encodeURIComponent(new URL(targetUrl).hostname) : "");
  }

  return { userAgent, acceptLanguage, referer, viewport, headers };
}

/**
 * Apply stealth patches to a Playwright page.
 * Hides automation indicators that websites use to detect bots.
 */
export async function applyStealthToPage(page: any, profile: StealthProfile): Promise<void> {
  // Override navigator.webdriver
  await page.addInitScript(() => {
    Object.defineProperty(navigator, 'webdriver', { get: () => false });

    // Hide automation-related properties
    const originalQuery = window.navigator.permissions.query;
    // @ts-ignore
    window.navigator.permissions.query = (parameters: any) =>
      parameters.name === 'notifications'
        ? Promise.resolve({ state: Notification.permission } as PermissionStatus)
        : originalQuery(parameters);

    // Spoof plugins (Chrome shows plugins, headless doesn't)
    Object.defineProperty(navigator, 'plugins', {
      get: () => [1, 2, 3, 4, 5],
    });

    // Spoof languages
    Object.defineProperty(navigator, 'languages', {
      get: () => ['en-US', 'en'],
    });

    // Chrome specific: hide chrome.runtime
    // @ts-ignore
    window.chrome = { runtime: {} };

    // Hide headless indicators in WebGL
    const getParameter = WebGLRenderingContext.prototype.getParameter;
    WebGLRenderingContext.prototype.getParameter = function(parameter: number) {
      if (parameter === 37445) return 'Intel Inc.';
      if (parameter === 37446) return 'Intel Iris OpenGL Engine';
      return getParameter.call(this, parameter);
    };
  });
}

/**
 * Add a random delay between requests to avoid rate limiting.
 * Mimics human browsing patterns.
 */
export async function humanDelay(action: string = "request"): Promise<void> {
  // Different delays for different actions
  switch (action) {
    case "page_load":
      await randomDelay(1000, 3000); // 1-3s after page load (reading time)
      break;
    case "click":
      await randomDelay(500, 1500); // 0.5-1.5s between clicks
      break;
    case "scroll":
      await randomDelay(200, 800); // 0.2-0.8s between scrolls
      break;
    case "request":
    default:
      await randomDelay(800, 2500); // 0.8-2.5s between page requests
      break;
  }
}

/**
 * Create a Playwright browser context with full stealth configuration.
 */
export async function createStealthContext(browser: any, profile?: StealthProfile): Promise<any> {
  const p = profile || generateProfile();

  const context = await browser.newContext({
    userAgent: p.userAgent,
    viewport: p.viewport,
    locale: p.acceptLanguage.split(",")[0].split(";")[0],
    timezoneId: pick(["America/New_York", "America/Los_Angeles", "Europe/London", "Asia/Tokyo", "Asia/Kolkata"]),
    geolocation: undefined,
    permissions: [],
    extraHTTPHeaders: {
      "Accept-Language": p.acceptLanguage,
    },
    // Realistic screen size
    screen: { width: p.viewport.width, height: p.viewport.height },
    hasTouch: false,
    isMobile: false,
    javaScriptEnabled: true,
  });

  return context;
}
