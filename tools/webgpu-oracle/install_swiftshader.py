"""Installs SwiftShader's Vulkan driver from Chrome for Testing
147.0.7727.50, the rasterizer of the WebGPU oracle (docs/webgpu-oracle.md),
for samples/three-render's test.

    python -I tools/webgpu-oracle/install_swiftshader.py DIR

Downloads chrome-headless-shell for this platform and extracts the driver,
its manifest and (on Windows) the Vulkan loader into DIR, unless DIR
already holds them. Each file must match its pinned SHA-256: these are the
files of the Chrome for Testing build the oracle was recorded with.
Prints the environment the test needs: VK_DRIVER_FILES and
VK_ICD_FILENAMES (older loaders) name the manifest. On Windows, DIR must
also be on PATH so that wgpu loads this vulkan-1.dll. On Linux the
system's loader (libvulkan1) loads the driver.
"""

import hashlib
import io
import pathlib
import sys
import urllib.request
import zipfile

VERSION = "147.0.7727.50"
FILES = {
    "linux64": {
        "libvk_swiftshader.so": "a2c809468d66c2363ad02778c1178712f638b5b67d1b91cb3f29ee6c8f930035",
        "vk_swiftshader_icd.json": "c9eef28b6b984fec220ef0abfecc40b502d46946706e47bfe97707027cb818bd",
    },
    "win64": {
        "vk_swiftshader.dll": "f3000e89a0e826637f9d2d393bbaad908c87fc6dce2f0f0b104e7d077e42f686",
        "vk_swiftshader_icd.json": "32d83ff113fef532a9f97e0d2831f8656628ab1c99e9060f0332b1532839afd9",
        "vulkan-1.dll": "28b1eb05d4b3231050d7bb4e90bcc5e9da6e14969175f4249e04753f4ce1edc7",
    },
}


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: python -I tools/webgpu-oracle/install_swiftshader.py DIR")
    out = pathlib.Path(sys.argv[1]).resolve()
    platform = {"linux": "linux64", "win32": "win64"}.get(sys.platform)
    if platform is None:
        sys.exit(f"no SwiftShader files pinned for {sys.platform}")
    files = FILES[platform]
    present = all((out / name).is_file() and sha256((out / name).read_bytes()) == digest for name, digest in files.items())
    if not present:
        url = f"https://storage.googleapis.com/chrome-for-testing-public/{VERSION}/{platform}/chrome-headless-shell-{platform}.zip"
        print(f"downloading {url}", file=sys.stderr)
        with urllib.request.urlopen(url) as response:
            archive = zipfile.ZipFile(io.BytesIO(response.read()))
        out.mkdir(parents=True, exist_ok=True)
        for name, digest in files.items():
            data = archive.read(f"chrome-headless-shell-{platform}/{name}")
            if sha256(data) != digest:
                sys.exit(f"{name}: SHA-256 {sha256(data)}, expected {digest}")
            (out / name).write_bytes(data)
    manifest = out / "vk_swiftshader_icd.json"
    print(f"VK_DRIVER_FILES={manifest}")
    print(f"VK_ICD_FILENAMES={manifest}")


if __name__ == "__main__":
    main()
