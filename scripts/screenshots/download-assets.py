"""Verify or restore the credited, screenshot-only wallpaper JPEGs."""

import argparse
import hashlib
import io
import json
from pathlib import Path

import requests
from bs4 import BeautifulSoup
from PIL import Image, ImageOps


BASE = Path(__file__).resolve().parent
ASSETS = BASE / "assets"
USER_AGENT = (
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
    "AppleWebKit/537.36 Chrome/120.0.0.0 Safari/537.36"
)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def asset_path(item):
    path = (ASSETS / item["filename"]).resolve()
    if path.parent != ASSETS.resolve():
        raise ValueError("Asset filenames must name a file directly inside assets/.")
    return path


def verify_asset(item):
    path = asset_path(item)
    if not path.is_file():
        return False
    if digest(path.read_bytes()) != item["sha256"]:
        return False
    with Image.open(path) as image:
        return image.size == (item["width"], item["height"])


def restore_asset(item):
    with requests.Session() as session:
        session.headers["User-Agent"] = USER_AGENT
        page = session.get(item["sourceUrl"], timeout=45)
        page.raise_for_status()
        soup = BeautifulSoup(page.text, "html.parser")
        metadata_tag = soup.select_one('script[type="application/ld+json"]')
        if metadata_tag is None:
            raise ValueError("The source page no longer exposes photo metadata.")
        metadata = json.loads(metadata_tag.string)
        if metadata.get("author") != item["author"]:
            raise ValueError("The source photographer differs from the manifest.")
        if metadata.get("license") != item["licenseUrl"]:
            raise ValueError("The source license differs from the manifest.")
        form = soup.select_one('form[action="/photo/download"]')
        if form is None:
            raise ValueError("The source page no longer exposes its download form.")
        fields = {field["name"]: field["value"] for field in form.select("input")}
        if fields.get("photoId") != item["sourcePhotoId"]:
            raise ValueError("The download form points to a different photo.")
        response = session.post(
            "https://stocksnap.io/photo/download", data=fields, timeout=60
        )
        response.raise_for_status()

    if digest(response.content) != item["sourceSha256"]:
        raise ValueError("The downloaded original differs from the recorded source.")

    settings = item["processing"]
    with Image.open(io.BytesIO(response.content)) as original:
        if original.size != (item["sourceWidth"], item["sourceHeight"]):
            raise ValueError("The original dimensions differ from the manifest.")
        export = ImageOps.fit(
            ImageOps.exif_transpose(original).convert("RGB"),
            (settings["width"], settings["height"]),
            method=Image.Resampling.LANCZOS,
            centering=(0.5, 0.5),
        )
    encoded = io.BytesIO()
    export.save(
        encoded,
        format="JPEG",
        quality=settings["jpegQuality"],
        optimize=settings["optimize"],
        progressive=settings["progressive"],
    )
    data = encoded.getvalue()
    if digest(data) != item["sha256"]:
        raise ValueError("JPEG output differs. Use Pillow 12.2.0 and recheck locally.")
    ASSETS.mkdir(exist_ok=True)
    asset_path(item).write_bytes(data)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify", action="store_true", help="Check local files only.")
    args = parser.parse_args()
    manifest = json.loads((BASE / "assets.json").read_text(encoding="utf-8"))
    failed = []
    for item in manifest["wallpapers"]:
        name = item["filename"]
        try:
            if not verify_asset(item):
                if args.verify:
                    raise ValueError("Missing asset, changed SHA-256, or wrong dimensions.")
                restore_asset(item)
            print(f"OK {name}")
        except (OSError, ValueError, requests.RequestException) as error:
            failed.append(name)
            print(f"FAIL {name}: {error}")
    if failed:
        raise SystemExit(1)
    print(f"Verified {len(manifest['wallpapers'])} screenshot assets.")


if __name__ == "__main__":
    main()
