// Records a walkthrough of the housing-web UI as an MP4-ready frame sequence.
// Usage (needs Google Chrome and Playwright, run as a local tool, not a project dependency):
//   npm i --prefix /tmp/rec playwright@1.64.1 && cp scripts/record_demo.mjs /tmp/rec/ && node /tmp/rec/record_demo.mjs /tmp/rec/out
//   (ES modules resolve imports next to the script, so it runs from the folder holding node_modules.)
//   ffmpeg -f concat -safe 0 -i /tmp/rec/out/frames/list.txt -vf "scale=1280:-2,fps=30,format=yuv420p" demo.mp4
// Start the UI first: cargo run --release [--features llm] --manifest-path agents/Cargo.toml --bin housing-web
//
// Links open in a new tab; the recorder follows them and films the source
// page with a caption showing its address (screencasts have no address bar).
// The wait for the live models is sped up 8x in the frame list.
import { chromium } from "playwright";
import { mkdirSync, writeFileSync } from "node:fs";

const out = process.argv[2] || "videos";
const SPEEDUP = 8;
const marks = {};
const mark = (label) => { marks[label] = Date.now() / 1000; console.log(`MARK ${label}`); };
const pause = (ms) => new Promise((r) => setTimeout(r, ms));

// A visible window, because headless Chrome can't display PDFs (most papers are PDFs).
const browser = await chromium.launch({ channel: "chrome", headless: false, args: ["--window-size=1280,900"] });
const context = await browser.newContext({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1 });
const page = await context.newPage();

// Capture frames with Chrome's screencast (no extra binaries); keep timestamps.
mkdirSync(`${out}/frames`, { recursive: true });
const frames = [];
let cdp;
async function film(p) {
  if (cdp) await cdp.send("Page.stopScreencast").catch(() => {});
  cdp = await context.newCDPSession(p);
  const session = cdp;
  session.on("Page.screencastFrame", async ({ data, metadata, sessionId }) => {
    if (session !== cdp) return;
    const file = `${out}/frames/${String(frames.length).padStart(5, "0")}.jpg`;
    writeFileSync(file, Buffer.from(data, "base64"));
    frames.push({ file, t: metadata.timestamp });
    await session.send("Page.screencastFrameAck", { sessionId }).catch(() => {});
  });
  await session.send("Page.startScreencast", { format: "jpeg", quality: 90, maxWidth: 1280, maxHeight: 800, everyNthFrame: 1 });
}

// A caption bar at the bottom of whatever page is on screen.
const caption = (p, text) => p.evaluate((text) => {
  let el = document.getElementById("__demo_caption");
  if (!el) {
    el = document.createElement("div");
    el.id = "__demo_caption";
    el.style.cssText = "position:fixed;left:50%;bottom:22px;transform:translateX(-50%);max-width:1100px;z-index:2147483647;" +
      "background:rgba(20,24,34,.92);color:#fff;font:600 17px/1.4 -apple-system,Segoe UI,sans-serif;padding:10px 18px;border-radius:10px;" +
      "box-shadow:0 6px 24px rgba(0,0,0,.25);text-align:center;pointer-events:none";
    document.body.appendChild(el);
  }
  el.innerHTML = text;
  el.style.display = text ? "block" : "none";
}, text).catch(() => {});

const smooth = (y) => page.evaluate((y) => window.scrollTo({ top: y, behavior: "smooth" }), y);
const scrollTo = async (sel, offset = 90) => {
  const y = await page.evaluate(([sel, offset]) => document.querySelector(sel).getBoundingClientRect().top + window.scrollY - offset, [sel, offset]);
  await smooth(y);
};

// Click a link that opens a new tab, film the source page, then come back.
async function follow(selector, label) {
  const link = page.locator(selector).first();
  await link.scrollIntoViewIfNeeded();
  await link.hover();
  await pause(900);
  const [popup] = await Promise.all([context.waitForEvent("page"), link.click()]);
  await popup.waitForLoadState("domcontentloaded", { timeout: 45000 }).catch(() => {});
  await popup.waitForLoadState("networkidle", { timeout: 20000 }).catch(() => {});
  await pause(1500);
  await film(popup);
  await caption(popup, `${label}<br><span style="font-weight:400;font-size:14px;opacity:.85">${popup.url()}</span>`);
  await pause(4200);
  await popup.evaluate(() => window.scrollTo({ top: 500, behavior: "smooth" })).catch(() => {});
  await pause(2200);
  await film(page);
  await popup.close();
  await pause(600);
}

await film(page);
mark("start");
await page.goto("http://127.0.0.1:8787/?state=PA");
await page.waitForSelector("tr.scenario");
await caption(page, "Pennsylvania: permits forecast county by county for 2026-2030, with starts and completions by year");
await pause(4000);

// The baseline explainer and back-test.
await scrollTo(".explainer", 300);
await caption(page, "Plain-language method, and how the model did when re-run on past years");
await pause(5000);

// Data-source links go to the publisher's page.
await caption(page, "Every data source links to its publisher");
await pause(1500);
await follow('.explainer a[href*="construction/bps"]', "Census Building Permits Survey: the permits behind the baseline");
await follow('.explainer a[href*="construction/nrc"]', "Census Survey of Construction: how long permits take to become homes");
await follow('.explainer a[href*="fhfa.gov"]', "FHFA House Price Index: county price growth");

// What if the economy is worse?
await scrollTo(".controls", 360);
await caption(page, "What if the economy is worse? Raise interest rates and cut migration");
await pause(1500);
const slide = (id, v) => page.evaluate(([id, v]) => { const el = document.getElementById(id); el.value = v; el.dispatchEvent(new Event("input")); }, [id, v]);
for (const v of ["0.5", "1", "1.5", "2"]) { await slide("rate", v); await pause(450); }
for (const v of ["0.8", "0.6", "0.5"]) { await slide("migration", v); await pause(450); }
await smooth(0);
await pause(3500);
await page.click("#reset");
await caption(page, "Back to recent conditions");
await pause(2000);

// Open the ADU scenario's explanation.
await scrollTo("#scenarios", 160);
await caption(page, "Click a reform to see what drives it");
await pause(1800);
await page.click("tr.scenario[data-k='0']");
await pause(1200);
await scrollTo("#explain-0", 130);
await caption(page, "The reform, the result in plain words, and extra homes by year");
await pause(5000);
await scrollTo("#explain-0 .grid2", 110);
await caption(page, "Which assumptions move the number most, and how it is calculated");
await pause(5000);
await scrollTo("#explain-0 table.assume", 110);
await caption(page, "Every assumption: low, central and high value, whether it comes from a study, and why");
await pause(5500);

// A study citation goes to the paper.
await caption(page, "Citations link to the research itself");
await pause(1200);
await follow('#explain-0 table.assume a.cite[href*="furmancenter"]', "Marantz, Elmendorf & Kim (NYU Furman Center): evidence behind the legalization and implementation assumptions");
await scrollTo("#explain-0 .detail > ul", 160);
await caption(page, "And what the estimate cannot capture");
await pause(3500);
await page.click("tr.scenario[data-k='0']");
await pause(600);

// Ask the expert panel (live models).
await scrollTo("#question", 200);
await caption(page, "Ask the expert panel: live Apache-2.0 models on OpenRouter");
await page.fill("#question", "");
await page.type("#question", "If Pennsylvania passes a statewide ADU law, how many homes could it add in five years?", { delay: 25 });
await pause(800);
await page.click("#ask");
mark("ask_clicked");
await caption(page, `The leader routes the question to domain experts (sped up ${SPEEDUP}x; the timer shows real time)`);
await page.waitForSelector("#result .answer, #result .check", { timeout: 300000 });
mark("answered");
await caption(page, "Experts are named, and study ids become linked citations");
await pause(1500);

const y = await page.evaluate(() => document.querySelector("#result").getBoundingClientRect().top + window.scrollY - 80);
for (let k = 0; k < 5; k++) { await smooth(y + k * 420); await pause(1900); }
// Some publishers (pew.org) put automated browsers behind a bot check; follow another citation.
const answerCite = '#result a.cite:not([href*="pew.org"])';
if (await page.locator(answerCite).count()) {
  const id = await page.locator(answerCite).first().textContent();
  await follow(answerCite, `${id}: a study the expert cited`);
}
await caption(page, "Every figure is checked against what the tools returned; the permission audit trail is below");
await page.evaluate(() => document.querySelector("#result details")?.setAttribute("open", ""));
await smooth(await page.evaluate(() => document.body.scrollHeight));
await pause(4000);
mark("end");

await cdp.send("Page.stopScreencast");
// ffmpeg concat list: each frame shown until the next one arrived; the
// model wait plays at SPEEDUP x.
const lines = [];
frames.forEach((f, i) => {
  const next = frames[i + 1]?.t ?? f.t + 1.5;
  let d = Math.max(0.02, next - f.t);
  if (f.t >= marks.ask_clicked + 1 && f.t < marks.answered) d /= SPEEDUP;
  lines.push(`file '${f.file.split("/").pop()}'`, `duration ${d.toFixed(3)}`);
});
lines.push(`file '${frames.at(-1).file.split("/").pop()}'`);
writeFileSync(`${out}/frames/list.txt`, lines.join("\n") + "\n");
console.log(`FRAMES ${frames.length}`);
await context.close();
await browser.close();
