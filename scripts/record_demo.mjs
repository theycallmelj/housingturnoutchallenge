// Records a walkthrough of the housing-web UI as an MP4-ready frame sequence.
// Usage (needs Google Chrome and Playwright, run as a local tool, not a project dependency):
//   npm i --prefix /tmp/rec playwright@1.64.1 && cp scripts/record_demo.mjs /tmp/rec/ && node /tmp/rec/record_demo.mjs /tmp/rec/out
//   (ES modules resolve imports next to the script, so it runs from the folder holding node_modules.)
//   ffmpeg -f concat -safe 0 -i /tmp/rec/out/frames/list.txt -vf "scale=1280:-2,fps=30,format=yuv420p" demo.mp4
// Start the UI first: cargo run --release [--features llm] --manifest-path agents/Cargo.toml --bin housing-web
import { chromium } from "playwright";
import { mkdirSync, writeFileSync } from "node:fs";

const out = process.argv[2] || "videos";
const t0 = Date.now();
const mark = (label) => console.log(`MARK ${label} ${((Date.now() - t0) / 1000).toFixed(2)}`);
const pause = (ms) => new Promise((r) => setTimeout(r, ms));

const browser = await chromium.launch({ channel: "chrome", headless: true });
const context = await browser.newContext({
  viewport: { width: 1280, height: 800 },
  deviceScaleFactor: 1,
});
const page = await context.newPage();
// Capture frames with Chrome's screencast (no extra binaries); keep timestamps.
mkdirSync(`${out}/frames`, { recursive: true });
const frames = [];
const cdp = await context.newCDPSession(page);
cdp.on("Page.screencastFrame", async ({ data, metadata, sessionId }) => {
  const file = `${out}/frames/${String(frames.length).padStart(5, "0")}.jpg`;
  writeFileSync(file, Buffer.from(data, "base64"));
  frames.push({ file, t: metadata.timestamp });
  await cdp.send("Page.screencastFrameAck", { sessionId }).catch(() => {});
});
await cdp.send("Page.startScreencast", { format: "jpeg", quality: 90, maxWidth: 1280, maxHeight: 800, everyNthFrame: 1 });
const smooth = (y) => page.evaluate((y) => window.scrollTo({ top: y, behavior: "smooth" }), y);

mark("start");
await page.goto("http://127.0.0.1:8787/?state=PA");
await page.waitForSelector("tr.scenario");
await pause(2500);

// Compare states: California, then back to Pennsylvania.
await page.selectOption("#state", "CA");
await pause(2500);
await page.selectOption("#state", "PA");
await pause(2000);

// Open the ADU scenario's explanation: formula, sensitivity, sources.
await page.click("tr.scenario[data-k='0']");
await pause(1200);
await smooth(330);
await pause(2500);
await page.evaluate(() => document.querySelector("#explain-0 pre").scrollTo({ top: 260, behavior: "smooth" }));
await pause(2500);
await page.evaluate(() => document.querySelector("#explain-0 pre").scrollTo({ top: 620, behavior: "smooth" }));
await pause(2500);
await page.click("tr.scenario[data-k='0']");
await pause(800);

// Fee scenario.
await page.click("tr.scenario[data-k='4']");
await pause(2500);
await page.click("tr.scenario[data-k='4']");
await pause(600);

// Ask the expert panel (live models).
await smooth(document_height_guess());
function document_height_guess() { return 900; }
await pause(1200);
await page.fill("#question", "");
await page.type("#question", "If Pennsylvania passes a statewide ADU law, how many homes could it add in five years?", { delay: 25 });
await pause(800);
await page.click("#ask");
mark("ask_clicked");
await page.waitForSelector("#result .answer, #result .check", { timeout: 300000 });
mark("answered");
await pause(1500);

// Read the answer, then open the audit trail.
const y = await page.evaluate(() => document.querySelector("#result").getBoundingClientRect().top + window.scrollY - 80);
for (let k = 0; k < 6; k++) {
  await smooth(y + k * 420);
  await pause(1800);
}
await page.evaluate(() => document.querySelector("details")?.setAttribute("open", ""));
await smooth(await page.evaluate(() => document.body.scrollHeight));
await pause(3500);
mark("end");

await cdp.send("Page.stopScreencast");
// ffmpeg concat list: each frame shown until the next one arrived.
const lines = [];
frames.forEach((f, i) => {
  const next = frames[i + 1]?.t ?? f.t + 1.5;
  lines.push(`file '${f.file.split("/").pop()}'`, `duration ${Math.max(0.02, next - f.t).toFixed(3)}`);
});
lines.push(`file '${frames.at(-1).file.split("/").pop()}'`);
writeFileSync(`${out}/frames/list.txt`, lines.join("\n") + "\n");
console.log(`FRAMES ${frames.length} first_ts ${frames[0].t}`);
await context.close();
await browser.close();
