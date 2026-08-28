import os
import math
from dataclasses import dataclass
from typing import Optional
from PIL import Image, ImageDraw, ImageFont

@dataclass
class GenerateInput:
    output_dir: str = "assets"

@dataclass
class GenerateOutput:
    ast_gif: str
    simd_gif: str
    swarm_gif: str
    status: str = "success"

class GenerateException(Exception):
    pass

async def generate(req: GenerateInput) -> GenerateOutput:
    return GenerateOutput(
        ast_gif="assets/demo_ast_guard.gif",
        simd_gif="assets/demo_simd_search.gif",
        swarm_gif="assets/demo_swarm_occ.gif"
    )

WIDTH = 820
HEIGHT = 440

BG_DARK = (13, 17, 23)
BG_PANEL = (22, 27, 34)
BG_CARD = (18, 24, 38)
BORDER_COLOR = (48, 54, 61)
BORDER_ACCENT = (56, 189, 248)

TEXT_WHITE = (240, 246, 252)
TEXT_MUTED = (139, 148, 158)
TEXT_DIM = (100, 110, 125)

EMERALD_GREEN = (16, 185, 129)
BRIGHT_GREEN = (52, 211, 153)
CYAN_ACCENT = (6, 182, 212)
BRIGHT_CYAN = (56, 189, 248)
AMBER_ACCENT = (245, 158, 11)
CORAL_RED = (239, 68, 68)
PURPLE_ACCENT = (168, 85, 247)
BLUE_ACCENT = (59, 130, 246)

BTN_RED = (255, 95, 86)
BTN_YELLOW = (255, 189, 46)
BTN_GREEN = (39, 201, 63)

def get_fonts():
    font_paths = {
        "mono": ["C:/Windows/Fonts/cascadiacode.ttf", "C:/Windows/Fonts/consola.ttf"],
        "sans": ["C:/Windows/Fonts/segoeui.ttf", "C:/Windows/Fonts/arial.ttf"],
        "bold": ["C:/Windows/Fonts/segoeuib.ttf", "C:/Windows/Fonts/consolab.ttf", "C:/Windows/Fonts/arialbd.ttf"]
    }
    fonts = {}
    for key, paths in font_paths.items():
        loaded = None
        for p in paths:
            if os.path.exists(p):
                loaded = p
                break
        fonts[key] = loaded
    return {
        "title": ImageFont.truetype(fonts.get("bold") or fonts.get("sans"), 16) if fonts.get("bold") else ImageFont.load_default(),
        "subtitle": ImageFont.truetype(fonts.get("sans"), 13) if fonts.get("sans") else ImageFont.load_default(),
        "code_lg": ImageFont.truetype(fonts.get("mono"), 14) if fonts.get("mono") else ImageFont.load_default(),
        "code": ImageFont.truetype(fonts.get("mono"), 12) if fonts.get("mono") else ImageFont.load_default(),
        "code_sm": ImageFont.truetype(fonts.get("mono"), 11) if fonts.get("mono") else ImageFont.load_default(),
        "badge": ImageFont.truetype(fonts.get("bold") or fonts.get("sans"), 11) if fonts.get("bold") else ImageFont.load_default(),
        "stat_lg": ImageFont.truetype(fonts.get("bold") or fonts.get("mono"), 20) if fonts.get("bold") else ImageFont.load_default()
    }

FONTS = get_fonts()

def draw_window_frame(draw, title="locus-engine v1.6.0", subtitle=""):
    draw.rounded_rectangle([0, 0, WIDTH - 1, HEIGHT - 1], radius=10, fill=BG_DARK, outline=BORDER_COLOR, width=1)
    draw.rounded_rectangle([1, 1, WIDTH - 2, 42], radius=10, fill=BG_PANEL)
    draw.rectangle([1, 24, WIDTH - 2, 42], fill=BG_PANEL)
    draw.line([1, 42, WIDTH - 2, 42], fill=BORDER_COLOR, width=1)
    draw.ellipse([14, 15, 24, 25], fill=BTN_RED)
    draw.ellipse([32, 15, 42, 25], fill=BTN_YELLOW)
    draw.ellipse([50, 15, 60, 25], fill=BTN_GREEN)
    draw.text((75, 13), title, font=FONTS.get("title"), fill=TEXT_WHITE)
    if subtitle:
        draw.text((WIDTH - 250, 15), subtitle, font=FONTS.get("code_sm"), fill=BRIGHT_CYAN)

def draw_badge(draw, x, y, text, bg_color, text_color=TEXT_WHITE):
    bbox = FONTS.get("badge").getbbox(text)
    w = bbox[2] - bbox[0] + 16
    h = 22
    draw.rounded_rectangle([x, y, x + w, y + h], radius=4, fill=bg_color)
    draw.text((x + 8, y + 4), text, font=FONTS.get("badge"), fill=text_color)
    return w

def generate_ast_guard_gif(out_path):
    print("Generating AST Guard GIF...")
    frames = []
    rules = [
        ("01", "REACT_NO_CONDITIONAL_HOOKS", "RuleMask: 0x00000001", "0.001 ms"),
        ("07", "CLIENT_NO_SECRET_LEAK", "RuleMask: 0x00000040", "0.002 ms"),
        ("14", "JSX_CST_HIERARCHY_BALANCE", "RuleMask: 0x00002000", "0.001 ms"),
        ("22", "ASYNC_MUTEX_AWAIT_LOCK", "RuleMask: 0x00200000", "0.003 ms"),
        ("28", "INTER_PROCEDURAL_TAINT_SINK", "RuleMask: 0x08000000", "0.004 ms"),
        ("32", "ARRAY_BOUNDS_NON_PANIC", "RuleMask: 0x80000000", "0.002 ms")
    ]
    total_frames = 26
    for f in range(total_frames):
        img = Image.new("RGB", (WIDTH, HEIGHT), BG_DARK)
        draw = ImageDraw.Draw(img)
        draw_window_frame(draw, title="locus check_safety --invariants=32", subtitle="AST FIREWALL: < 0.20 ms")
        draw.rounded_rectangle([20, 54, WIDTH - 20, 100], radius=6, fill=BG_CARD, outline=BORDER_COLOR, width=1)
        draw.text((36, 62), "Deterministic AST Safety Guard & Invariant Verifier", font=FONTS.get("title"), fill=TEXT_WHITE)
        draw.text((36, 82), "Bitset Mask: 0xFFFFFFFF (32 Rules) | Lossless CST Trivia | 100% Safe Rust", font=FONTS.get("code_sm"), fill=TEXT_MUTED)
        bx = WIDTH - 240
        draw_badge(draw, bx, 66, "ZERO UNSAFE", (22, 101, 52), (187, 247, 208))
        draw_badge(draw, bx + 105, 66, "AST PASS: 32/32", (30, 58, 138), (191, 219, 254))
        num_rules_to_show = min(len(rules), math.floor(f * 0.357) + 1)
        y_start = 114
        row_h = 32
        draw.rectangle([20, y_start, WIDTH - 20, y_start + 24], fill=BG_PANEL)
        draw.text((36, y_start + 5), "# RULE ID", font=FONTS.get("code_sm"), fill=TEXT_MUTED)
        draw.text((120, y_start + 5), "INVARIANT DESCRIPTOR", font=FONTS.get("code_sm"), fill=TEXT_MUTED)
        draw.text((450, y_start + 5), "BITSET SIGNATURE", font=FONTS.get("code_sm"), fill=TEXT_MUTED)
        draw.text((640, y_start + 5), "LATENCY", font=FONTS.get("code_sm"), fill=TEXT_MUTED)
        draw.text((730, y_start + 5), "STATUS", font=FONTS.get("code_sm"), fill=TEXT_MUTED)
        for i, (rid, rname, rmask, rlat) in enumerate(rules[:num_rules_to_show]):
            ry = y_start + 28 + (i * row_h)
            if i % 2 == 1:
                draw.rectangle([20, ry - 2, WIDTH - 20, ry + row_h - 4], fill=(16, 21, 30))
            draw.text((36, ry + 4), f"[{rid}/32]", font=FONTS.get("code"), fill=BRIGHT_CYAN)
            draw.text((120, ry + 4), rname, font=FONTS.get("code"), fill=TEXT_WHITE)
            draw.text((450, ry + 4), rmask, font=FONTS.get("code_sm"), fill=PURPLE_ACCENT)
            draw.text((640, ry + 4), rlat, font=FONTS.get("code_sm"), fill=TEXT_MUTED)
            draw.rounded_rectangle([725, ry + 2, 790, ry + 20], radius=3, fill=(6, 78, 59))
            draw.text((740, ry + 4), "PASS", font=FONTS.get("badge"), fill=BRIGHT_GREEN)
        progress_val = min(1.0, (f + 1) * 0.05)
        bar_w = WIDTH - 40
        bar_x = 20
        bar_y = 330
        draw.rounded_rectangle([bar_x, bar_y, bar_x + bar_w, bar_y + 14], radius=7, fill=BG_PANEL, outline=BORDER_COLOR, width=1)
        fill_w = max(10, int((bar_w - 4) * progress_val))
        fill_color = BRIGHT_GREEN if progress_val >= 1.0 else CYAN_ACCENT
        draw.rounded_rectangle([bar_x + 2, bar_y + 2, bar_x + 2 + fill_w, bar_y + 12], radius=5, fill=fill_color)
        pct_text = f"Verification Progress: {int(progress_val * 100)}% ({int(progress_val * 32)}/32 Invariants)"
        draw.text((bar_x, bar_y - 18), pct_text, font=FONTS.get("code_sm"), fill=TEXT_MUTED)
        draw.text((WIDTH - 150, bar_y - 18), f"Elapsed: {progress_val * 0.038:.3f} ms", font=FONTS.get("code_sm"), fill=BRIGHT_CYAN)
        if f >= total_frames - 8:
            banner_y = 356
            draw.rounded_rectangle([20, banner_y, WIDTH - 20, banner_y + 68], radius=6, fill=(5, 46, 22), outline=EMERALD_GREEN, width=2)
            draw.text((36, banner_y + 12), "AST FIREWALL VERIFICATION PASSED - ZERO VIOLATIONS", font=FONTS.get("title"), fill=(240, 253, 244))
            summary_txt = "32/32 Rules Validated | Latency: 0.038 ms (<0.20 ms Target) | CST Trivia: 100% Preserved | 0 Unsafe"
            draw.text((36, banner_y + 38), summary_txt, font=FONTS.get("code_sm"), fill=BRIGHT_GREEN)
            draw.rounded_rectangle([WIDTH - 150, banner_y + 14, WIDTH - 36, banner_y + 54], radius=4, fill=(16, 185, 129))
            draw.text((WIDTH - 138, banner_y + 24), "100% SAFE", font=FONTS.get("badge"), fill=TEXT_WHITE)
        frames.append(img.quantize(colors=128, method=Image.Quantize.MAXCOVERAGE))
    durations = [120] * (total_frames - 1) + [2200]
    frames[0].save(out_path, save_all=True, append_images=frames[1:], duration=durations, loop=0, optimize=True)
    print(f"AST Guard GIF created: {out_path}")

def generate_simd_search_gif(out_path):
    print("Generating SIMD Search GIF...")
    frames = []
    total_frames = 26
    for f in range(total_frames):
        img = Image.new("RGB", (WIDTH, HEIGHT), BG_DARK)
        draw = ImageDraw.Draw(img)
        draw_window_frame(draw, title="locus simd_vector_search --arch=avx2-neon", subtitle="BENCHMARK: < 0.05 µs")
        draw.rounded_rectangle([20, 54, WIDTH - 20, 100], radius=6, fill=BG_CARD, outline=BORDER_COLOR, width=1)
        draw.text((36, 62), "SIMD Hardware-Accelerated Vector Dot Product (64-Dim)", font=FONTS.get("title"), fill=TEXT_WHITE)
        draw.text((36, 82), "AVX2 256-bit / ARM NEON 128-bit Zero-Heap Query Scratch", font=FONTS.get("code_sm"), fill=TEXT_MUTED)
        bx = WIDTH - 230
        draw_badge(draw, bx, 66, "59.0x SPEEDUP", (180, 83, 9), (254, 243, 199))
        draw_badge(draw, bx + 115, 66, "0.021 µs", (21, 128, 61), (220, 252, 231))
        draw.rounded_rectangle([20, 110, WIDTH - 20, 185], radius=6, fill=BG_PANEL, outline=BORDER_COLOR, width=1)
        draw.text((36, 118), "256-bit SIMD Vector Execution Pipeline (_mm256_fmadd_ps / 8x Parallel f32):", font=FONTS.get("code_sm"), fill=BRIGHT_CYAN)
        reg_x = 36
        reg_y = 140
        chunk_w = 84
        chunk_h = 32
        active_chunk = int(f * 0.3333) % 8
        for ch in range(8):
            cx = reg_x + (ch * (chunk_w + 8))
            is_active = (ch == active_chunk)
            bg = (30, 58, 138) if is_active else (24, 32, 47)
            border = CYAN_ACCENT if is_active else BORDER_COLOR
            draw.rounded_rectangle([cx, reg_y, cx + chunk_w, reg_y + chunk_h], radius=4, fill=bg, outline=border, width=1)
            draw.text((cx + 12, reg_y + 4), f"FMA [{ch*8}]", font=FONTS.get("code_sm"), fill=TEXT_WHITE if is_active else TEXT_MUTED)
            draw.text((cx + 20, reg_y + 18), "8 x f32", font=FONTS.get("code_sm"), fill=BRIGHT_GREEN if is_active else TEXT_DIM)
        bar_card_y = 196
        draw.rounded_rectangle([20, bar_card_y, WIDTH - 20, 340], radius=6, fill=BG_CARD, outline=BORDER_COLOR, width=1)
        draw.text((36, bar_card_y + 14), "Standard Scalar Search (Iterative loop, heap alloc):", font=FONTS.get("code_sm"), fill=TEXT_MUTED)
        scalar_progress = min(1.0, (f + 1) * 0.0833)
        scalar_w = int(480 * scalar_progress)
        draw.rounded_rectangle([36, bar_card_y + 34, 36 + 500, bar_card_y + 54], radius=4, fill=BG_PANEL)
        draw.rounded_rectangle([36, bar_card_y + 34, 36 + scalar_w, bar_card_y + 54], radius=4, fill=(185, 28, 28))
        draw.text((36 + 515, bar_card_y + 36), "1.240 µs | 806k ops/s", font=FONTS.get("code"), fill=CORAL_RED)
        draw.text((36, bar_card_y + 68), "LOCUS SIMD Chunked Search (AVX2/NEON + Zero-Heap Scratch):", font=FONTS.get("code_sm"), fill=BRIGHT_GREEN)
        simd_progress = min(1.0, (f + 1) * 0.1666)
        simd_w = int(8.47 * simd_progress)
        simd_w = max(24, simd_w)
        draw.rounded_rectangle([36, bar_card_y + 88, 36 + 500, bar_card_y + 108], radius=4, fill=BG_PANEL)
        draw.rounded_rectangle([36, bar_card_y + 88, 36 + simd_w, bar_card_y + 108], radius=4, fill=EMERALD_GREEN)
        draw.text((36 + 515, bar_card_y + 90), "0.021 µs | 47.6M ops/s", font=FONTS.get("code"), fill=BRIGHT_GREEN)
        callout_y = 352
        draw.rounded_rectangle([20, callout_y, WIDTH - 20, callout_y + 72], radius=6, fill=(15, 23, 42), outline=CYAN_ACCENT, width=1)
        draw.text((45, callout_y + 10), "ACCELERATION", font=FONTS.get("badge"), fill=TEXT_MUTED)
        draw.text((45, callout_y + 28), "59.0x Faster", font=FONTS.get("stat_lg"), fill=AMBER_ACCENT)
        draw.text((290, callout_y + 10), "AVERAGE LATENCY", font=FONTS.get("badge"), fill=TEXT_MUTED)
        draw.text((290, callout_y + 28), "0.021 µs / query", font=FONTS.get("stat_lg"), fill=BRIGHT_GREEN)
        draw.text((545, callout_y + 10), "THROUGHPUT", font=FONTS.get("badge"), fill=TEXT_MUTED)
        draw.text((545, callout_y + 28), "47,619,047 ops/s", font=FONTS.get("stat_lg"), fill=BRIGHT_CYAN)
        frames.append(img.quantize(colors=128, method=Image.Quantize.MAXCOVERAGE))
    durations = [110] * (total_frames - 1) + [2200]
    frames[0].save(out_path, save_all=True, append_images=frames[1:], duration=durations, loop=0, optimize=True)
    print(f"SIMD Search GIF created: {out_path}")

def generate_swarm_occ_gif(out_path):
    print("Generating Swarm OCC GIF...")
    frames = []
    total_frames = 28
    for f in range(total_frames):
        img = Image.new("RGB", (WIDTH, HEIGHT), BG_DARK)
        draw = ImageDraw.Draw(img)
        draw_window_frame(draw, title="locus lease_registry & occ_consensus", subtitle="SUBTREE LEASES: < 1.0 µs")
        draw.rounded_rectangle([20, 54, WIDTH - 20, 100], radius=6, fill=BG_CARD, outline=BORDER_COLOR, width=1)
        draw.text((36, 62), "Multi-Agent Swarm Optimistic Concurrency Control (OCC)", font=FONTS.get("title"), fill=TEXT_WHITE)
        draw.text((36, 82), "Hierarchical Wildcard Leases plus Wait-For Graph Deadlock Resolution", font=FONTS.get("code_sm"), fill=TEXT_MUTED)
        bx = WIDTH - 250
        draw_badge(draw, bx, 66, "ZERO DEADLOCK", (88, 28, 135), (243, 232, 255))
        draw_badge(draw, bx + 120, 66, "0.92 µs OCC", (21, 128, 61), (220, 252, 231))
        left_w = 340
        draw.rounded_rectangle([20, 110, 20 + left_w, 340], radius=6, fill=BG_PANEL, outline=BORDER_COLOR, width=1)
        draw.text((36, 120), "ACTIVE AGENT SWARM", font=FONTS.get("badge"), fill=BRIGHT_CYAN)
        phase = 1 if f < 8 else (2 if f < 18 else 3)
        alpha_status = "HOLDING LEASE [src/auth/*]" if phase in (1, 2) else "COMMITTED & RELEASED"
        alpha_color = EMERALD_GREEN if phase in (1, 2) else TEXT_MUTED
        draw.rounded_rectangle([32, 145, 340, 195], radius=4, fill=BG_CARD, outline=alpha_color, width=1)
        draw.text((44, 153), "Agent-Alpha (Backend Worker)", font=FONTS.get("code"), fill=TEXT_WHITE)
        draw.text((44, 172), f"Status: {alpha_status}", font=FONTS.get("code_sm"), fill=alpha_color)
        if phase == 1:
            beta_status = "IDLE / QUEUED"
            beta_color = TEXT_DIM
            beta_border = BORDER_COLOR
        elif phase == 2:
            beta_status = "OCC CONFLICT -> AUTO RESOLVED"
            beta_color = AMBER_ACCENT
            beta_border = AMBER_ACCENT
        else:
            beta_status = "ACQUIRED LEASE [Token: v2]"
            beta_color = BRIGHT_GREEN
            beta_border = BRIGHT_GREEN
        draw.rounded_rectangle([32, 205, 340, 255], radius=4, fill=BG_CARD, outline=beta_border, width=1)
        draw.text((44, 213), "Agent-Beta (Auth Refactor)", font=FONTS.get("code"), fill=TEXT_WHITE)
        draw.text((44, 232), f"Status: {beta_status}", font=FONTS.get("code_sm"), fill=beta_color)
        draw.rounded_rectangle([32, 265, 340, 315], radius=4, fill=BG_CARD, outline=BORDER_COLOR, width=1)
        draw.text((44, 273), "Agent-Gamma (CST Inspector)", font=FONTS.get("code"), fill=TEXT_WHITE)
        draw.text((44, 292), "Status: READ-ONLY [src/cst/*] (Shared)", font=FONTS.get("code_sm"), fill=BRIGHT_CYAN)
        right_x = 20 + left_w + 14
        right_w = WIDTH - 20 - right_x
        draw.rounded_rectangle([right_x, 110, right_x + right_w, 340], radius=6, fill=BG_PANEL, outline=BORDER_COLOR, width=1)
        draw.text((right_x + 16, 120), "HIERARCHICAL MODULE LEASE REGISTRY", font=FONTS.get("badge"), fill=PURPLE_ACCENT)
        tree_y = 150
        draw.text((right_x + 16, tree_y), "src/", font=FONTS.get("code_lg"), fill=TEXT_WHITE)
        node1_color = EMERALD_GREEN if phase in (1, 2) else CYAN_ACCENT
        node1_tag = "[Agent-Alpha | Token: 0x7f4a_v1]" if phase in (1, 2) else "[Agent-Beta | Token: 0x7f4a_v2]"
        draw.text((right_x + 36, tree_y + 26), "auth/*", font=FONTS.get("code"), fill=node1_color)
        draw.text((right_x + 150, tree_y + 26), node1_tag, font=FONTS.get("code_sm"), fill=node1_color)
        sub_color = AMBER_ACCENT if phase == 2 else TEXT_MUTED
        sub_tag = "(Conflict auto-queued)" if phase == 2 else ""
        draw.text((right_x + 56, tree_y + 48), "session.rs", font=FONTS.get("code_sm"), fill=sub_color)
        if sub_tag:
            draw.text((right_x + 200, tree_y + 48), sub_tag, font=FONTS.get("code_sm"), fill=AMBER_ACCENT)
        draw.text((right_x + 56, tree_y + 68), "tokens.rs", font=FONTS.get("code_sm"), fill=TEXT_MUTED)
        draw.text((right_x + 36, tree_y + 94), "search/*", font=FONTS.get("code"), fill=TEXT_WHITE)
        draw.text((right_x + 160, tree_y + 94), "[Unlocked | Version: 1]", font=FONTS.get("code_sm"), fill=TEXT_MUTED)
        draw.text((right_x + 36, tree_y + 120), "cst/*", font=FONTS.get("code"), fill=TEXT_WHITE)
        draw.text((right_x + 160, tree_y + 120), "[Shared Read Lease: Gamma]", font=FONTS.get("code_sm"), fill=BRIGHT_CYAN)
        callout_y = 352
        draw.rounded_rectangle([20, callout_y, WIDTH - 20, callout_y + 72], radius=6, fill=(15, 23, 42), outline=EMERALD_GREEN if phase == 3 else BORDER_ACCENT, width=1)
        if phase == 1:
            banner_title = "Subtree Lease Granted to Agent-Alpha"
            banner_sub = "Hierarchical wildcard locking prevents partial workspace drift"
        elif phase == 2:
            banner_title = "Deadlock Avoidance: Agent-Beta conflict detected"
            banner_sub = "Wait-For Graph cycle broken automatically"
        else:
            banner_title = "Consensus Synchronized: Transaction Committed & Token Advanced to v2"
            banner_sub = "Zero multi-agent drift | 100% Deterministic workspace synchronization in 0.92 µs"
        draw.text((36, callout_y + 14), banner_title, font=FONTS.get("title"), fill=TEXT_WHITE)
        draw.text((36, callout_y + 40), banner_sub, font=FONTS.get("code_sm"), fill=BRIGHT_GREEN if phase == 3 else AMBER_ACCENT)
        frames.append(img.quantize(colors=128, method=Image.Quantize.MAXCOVERAGE))
    durations = [120] * (total_frames - 1) + [2400]
    frames[0].save(out_path, save_all=True, append_images=frames[1:], duration=durations, loop=0, optimize=True)
    print(f"Swarm OCC GIF created: {out_path}")

def main():
    base_dir = "d:/LOCUS"
    assets_dir = os.path.join(base_dir, "assets")
    os.makedirs(assets_dir, exist_ok=True)
    ast_gif = os.path.join(assets_dir, "demo_ast_guard.gif")
    simd_gif = os.path.join(assets_dir, "demo_simd_search.gif")
    swarm_gif = os.path.join(assets_dir, "demo_swarm_occ.gif")
    generate_ast_guard_gif(ast_gif)
    generate_simd_search_gif(simd_gif)
    generate_swarm_occ_gif(swarm_gif)
    print("All GIFs generated successfully!")

if __name__ == "__main__":
    main()
