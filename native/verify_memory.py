#!/usr/bin/env python3
"""Measure peak Linux GUI RSS while rendering fixed-size synthetic libraries.

Run against old and new release binaries on the same desktop. Requires DISPLAY
or WAYLAND_DISPLAY. All app writes stay in a temporary home. No user mods touched.
"""
import argparse
import json
from pathlib import Path
import struct
import subprocess
import tempfile
import time
import zlib


def preview_png(edge):
    def chunk(kind, data):
        return (struct.pack('>I', len(data)) + kind + data
                + struct.pack('>I', zlib.crc32(kind + data)))
    row = b'\0' + bytes([30, 120, 95]) * edge
    return (b'\x89PNG\r\n\x1a\n'
            + chunk(b'IHDR', struct.pack('>IIBBBBB', edge, edge, 8, 2, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(row * edge)) + chunk(b'IEND', b''))


def measure(binary, count, edge, seconds):
    with tempfile.TemporaryDirectory(prefix='plumcrate-memory-') as directory:
        fixture = Path(directory)
        root = fixture / 'The Sims 4'
        home = fixture / 'home'
        home.mkdir()
        photo = preview_png(edge)
        for index in range(count):
            group = root / 'Mods' / f'Collection {index:05}'
            group.mkdir(parents=True)
            (group / 'mod.package').write_bytes(b'synthetic memory fixture')
            (group / 'preview.png').write_bytes(photo)
        capture = fixture / 'render.png'
        with tempfile.TemporaryFile() as log:
            process = subprocess.Popen([str(binary), '--home', str(home),
                                        '--root', str(root), '--screenshot', str(capture),
                                        '--size', '1200x800'], stdout=log, stderr=log)
            samples = []
            start = time.monotonic()
            try:
                while time.monotonic() - start < seconds:
                    if process.poll() is not None:
                        if process.returncode != 0:
                            log.seek(0)
                            raise RuntimeError(log.read().decode(errors='replace'))
                        break
                    try:
                        status = Path(f'/proc/{process.pid}/status').read_text()
                    except FileNotFoundError:
                        continue
                    rss = next((int(line.split()[1]) for line in status.splitlines()
                                if line.startswith('VmRSS:')), None)
                    if rss is not None:
                        samples.append(rss)
                    time.sleep(0.005)
            finally:
                process.terminate()
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
        if not capture.is_file():
            raise RuntimeError('App did not finish rendering before timeout')
        from verify import assert_preview_pixels
        assert_preview_pixels(capture)
        return {'mods': count, 'previewEdge': edge, 'window': '1200x800',
                'peakRssMiB': round(max(samples) / 1024, 2)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--sizes', type=int, nargs='+', default=[250, 500])
    parser.add_argument('--preview-edge', type=int, default=2048)
    parser.add_argument('--seconds', type=float, default=45, help='Render timeout per library')
    args = parser.parse_args()
    if min(args.sizes) < 1 or args.seconds < 2 or not 1 <= args.preview_edge <= 4096:
        parser.error('Use positive sizes, at least 2 seconds timeout, and image edges up to 4096')
    for count in args.sizes:
        print(json.dumps(measure(args.binary.resolve(), count,
                                 args.preview_edge, args.seconds)), flush=True)


if __name__ == '__main__':
    main()
