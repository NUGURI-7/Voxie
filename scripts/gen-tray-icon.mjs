/**
 * gen-tray-icon.mjs
 * 生成 Voxie 托盘图标（macOS template image）
 * 输出：src-tauri/icons/tray-icon.png（36×36，黑色叶子 V 形，透明背景）
 *      src-tauri/icons/tray-icon@2x.png（72×72 retina）
 *
 * 运行：node scripts/gen-tray-icon.mjs
 */

import { Resvg } from "@resvg/resvg-js";
import { writeFileSync, mkdirSync } from "fs";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";

const __dir = dirname(fileURLToPath(import.meta.url));
const iconDir = resolve(__dir, "../src-tauri/icons");

// ── SVG：粗描边叶子 V 形，viewBox 20×20，transparent 背景
// macOS template image 要求：纯黑色 + 透明；系统自动适配深浅色菜单栏
// 在 18px 下需要足够粗，用 stroke+fill 保证可见性
const SVG = `<svg viewBox="-1.5 -1.5 23 23" fill="none" xmlns="http://www.w3.org/2000/svg">
  <!-- 左叶：加粗描边 + 填充，18px 下清晰可见 -->
  <path d="M10,17.5 C8,11.5 5,7 1.5,1.5 C5,4 7.5,8.5 10,17.5Z"
        fill="black" stroke="black" stroke-width="2.2"
        stroke-linejoin="round" stroke-linecap="round"/>
  <!-- 右叶 -->
  <path d="M10,17.5 C12,11.5 15,7 18.5,1.5 C15,4 12.5,8.5 10,17.5Z"
        fill="black" stroke="black" stroke-width="2.2"
        stroke-linejoin="round" stroke-linecap="round"/>
</svg>`;

function renderPng(svgStr, size) {
  const resvg = new Resvg(svgStr, {
    fitTo: { mode: "width", value: size },
    background: "transparent",
  });
  return resvg.render().asPng();
}

// 1x（18×18）macOS 菜单栏标准尺寸
const png1x = renderPng(SVG, 18);
writeFileSync(resolve(iconDir, "tray-icon.png"), png1x);
console.log("✅ tray-icon.png     (18×18)");

// 2x retina（36×36）
const png2x = renderPng(SVG, 36);
writeFileSync(resolve(iconDir, "tray-icon@2x.png"), png2x);
console.log("✅ tray-icon@2x.png  (36×36)");

console.log("\n托盘图标生成完成 → src-tauri/icons/");
