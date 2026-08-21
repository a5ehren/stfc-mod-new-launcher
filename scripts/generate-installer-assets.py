#!/usr/bin/env python3
"""Generate LCARS v26 (classic palette) installer assets:

  src-tauri/installer/dmg-background.png  660x400  macOS DMG window background
  src-tauri/installer/wix-banner.bmp      493x58   MSI top banner (all pages)
  src-tauri/installer/wix-dialog.bmp      493x312  MSI welcome/completion dialog

Requires: pillow (font files are produced from the woff2 in Downloads via
fonttools; see ANTONIO_TTF paths below).
"""

from PIL import Image, ImageDraw, ImageFont

# LCARS Classic v26 palette
BLACK = "#000000"
VIOLET = "#baa4e5"  # african-violet
ORANGE = "#eb943a"
ALMOND = "#d29b7f"
BLUEY = "#8899ff"
RED = "#cf4f4f"
BARLEY = "#edb378"
BUTTERSCOTCH = "#ea9c72"
BROWN = "#895129"
PUMPKIN_SEED = "#ffe1ca"

ANTONIO_BOLD = "/tmp/Antonio-Bold.ttf"
ANTONIO_REGULAR = "/tmp/Antonio-Regular.ttf"

OUT_DIR = "../src-tauri/installer"


def font(path: str, size: int) -> ImageFont.FreeTypeFont:
    return ImageFont.truetype(path, size)


def elbow(draw: ImageDraw.ImageDraw, x: int, y: int, bar: int, w: int, h: int,
          radius: int, color: str) -> None:
    """Draw an LCARS elbow anchored at (x, y): vertical bar going down `h`,
    horizontal bar going right `w`, with the inner corner rounded by `radius`."""
    draw.rectangle([x, y, x + bar, y + h], fill=color)          # vertical
    draw.rectangle([x, y, x + w, y + bar], fill=color)          # horizontal
    # carve the inner corner with a background-colored disc
    draw.ellipse([x + bar, y + bar, x + bar + 2 * radius, y + bar + 2 * radius],
                 fill=BLACK)


def text_right(draw: ImageDraw.ImageDraw, xy, text: str, f: ImageFont.FreeTypeFont,
               fill: str) -> None:
    x, y = xy
    width = draw.textlength(text, font=f)
    draw.text((x - width, y), text, font=f, fill=fill)


def dmg_background() -> None:
    W, H = 660, 400
    img = Image.new("RGB", (W, H), BLACK)
    d = ImageDraw.Draw(img)

    # Top bar: bluey, full width, rounded left cap
    d.rounded_rectangle([16, 16, W - 16, 48], radius=16, fill=BLUEY)
    d.rectangle([W - 32, 16, W - 16, 48], fill=BLUEY)  # square right end
    text_right(d, (W - 28, 20), "LCARS", font(ANTONIO_BOLD, 22), BLACK)

    # Left vertical panel (almond) + bottom-left elbow (red)
    d.rectangle([16, 56, 52, 260], fill=ALMOND)
    d.rounded_rectangle([16, 224, 52, 268], radius=12, fill=ALMOND)
    elbow(d, 16, 276, 36, 220, 76, 26, RED)
    text_right(d, (46, 96), "02", font(ANTONIO_BOLD, 18), BLACK)
    text_right(d, (46, 122), "262000", font(ANTONIO_BOLD, 18), BLACK)

    # Title block sits above the icon drop zones (icons: y=106..234)
    d.text((72, 58), "STFC COMMUNITY MOD LAUNCHER",
           font=font(ANTONIO_BOLD, 30), fill=ORANGE)
    d.text((72, 90), "STAR TREK FLEET COMMAND", font=font(ANTONIO_BOLD, 16),
           fill=VIOLET)

    # Arrow between the app icon (x=180) and Applications folder (x=480);
    # icon spans are roughly x=116..244 and x=416..544, so stay between them
    arrow_y = 163
    d.rounded_rectangle([256, arrow_y, 380, arrow_y + 14], radius=7, fill=ORANGE)
    d.polygon([(380, arrow_y - 7), (380, arrow_y + 21), (408, arrow_y + 7)],
              fill=ORANGE)
    label = "DRAG TO APPLICATIONS"
    label_font = font(ANTONIO_BOLD, 16)
    lw = d.textlength(label, font=label_font)
    d.text((330 - lw / 2, arrow_y + 30), label, font=label_font, fill=BUTTERSCOTCH)

    # Bottom segmented bar (echoes the in-app bar panel)
    bar_y, bar_h, gap = 356, 20, 8
    segments = [(72, 400, BLUEY), (408, 480, ORANGE), (488, 576, VIOLET),
                (584, 644, RED)]
    for x0, x1, color in segments:
        d.rectangle([x0, bar_y, x1, bar_y + bar_h], fill=color)
    d.text((72, bar_y - 24), "10-31", font=font(ANTONIO_BOLD, 16), fill=BARLEY)

    img.save(f"{OUT_DIR}/dmg-background.png")


def wix_banner() -> None:
    W, H = 493, 58
    img = Image.new("RGB", (W, H), BLACK)
    d = ImageDraw.Draw(img)

    d.rounded_rectangle([8, 8, 116, H - 8], radius=20, fill=VIOLET)
    d.rectangle([100, 8, 116, H - 8], fill=VIOLET)  # square right end of pill
    text_right(d, (108, 16), "LCARS", font(ANTONIO_BOLD, 20), BLACK)

    # WiX draws its own white dialog title over the rest of the banner, so
    # keep it plain black with only a few abstract accent segments far right.
    for i, color in enumerate([BLUEY, ORANGE, RED]):
        x = W - 40 + i * 12
        d.rectangle([x, 8, x + 8, H - 8], fill=color)

    img.save(f"{OUT_DIR}/wix-banner.bmp")


def wix_dialog() -> None:
    W, H = 493, 312
    img = Image.new("RGB", (W, H), PUMPKIN_SEED)  # light zone for overlay text
    d = ImageDraw.Draw(img)

    # Left LCARS zone (black) ~164px wide
    d.rectangle([0, 0, 160, H], fill=BLACK)

    # Top bar (violet) with label
    d.rounded_rectangle([8, 8, 152, 32], radius=12, fill=VIOLET)
    d.rectangle([140, 8, 152, 32], fill=VIOLET)
    text_right(d, (132, 10), "LCARS", font(ANTONIO_BOLD, 16), BLACK)

    # Vertical panel (almond) with stacked code numbers
    d.rectangle([8, 40, 32, 200], fill=ALMOND)
    text_right(d, (28, 52), "02", font(ANTONIO_BOLD, 14), BLACK)
    text_right(d, (28, 74), "26", font(ANTONIO_BOLD, 14), BLACK)

    # Bottom elbow (red)
    elbow(d, 8, 208, 24, 110, 66, 18, RED)

    # Stacked accent buttons on the right edge of the dark zone
    for i, color in enumerate([BLUEY, ORANGE, RED]):
        y = 60 + i * 40
        d.rounded_rectangle([120, y, 148, y + 28], radius=10, fill=color)

    # Bottom accent bars in the light zone
    segments = [(176, 300, BLUEY), (308, 372, ORANGE), (380, 477, VIOLET)]
    for x0, x1, color in segments:
        d.rectangle([x0, 288, x1, 300], fill=color)
    d.text((176, 258), "STFC COMMUNITY MOD LAUNCHER",
           font=font(ANTONIO_BOLD, 20), fill=BROWN)

    img.save(f"{OUT_DIR}/wix-dialog.bmp")


if __name__ == "__main__":
    dmg_background()
    wix_banner()
    wix_dialog()
    print("wrote installer assets to", OUT_DIR)
