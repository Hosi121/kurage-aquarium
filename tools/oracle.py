#!/usr/bin/env python3
"""Regenerate migration fixtures from the pinned Python release, never Rust.

Development only: Python 3.12, numpy==2.5.4, Pillow==13.0.0. This script is
not imported or invoked by either Rust executable or its build.
"""
import ast
import hashlib
import io
import json
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile

import numpy as np
import PIL
from PIL import Image, ImageFont

SOURCE = "c5d795a"
ROOT = Path(__file__).resolve().parents[1]


def main():
    assert sys.version_info[:2] == (3, 12)
    assert np.__version__ == "2.5.4"
    assert PIL.__version__ == "13.0.0"
    data = subprocess.check_output(["git", "archive", SOURCE, "src", "tests"] , cwd=ROOT)
    with tempfile.TemporaryDirectory(prefix="kurage-oracle-") as temp:
        with tarfile.open(fileobj=io.BytesIO(data)) as archive:
            archive.extractall(temp, filter="data")
        sys.path.insert(0, str(Path(temp) / "src"))
        from kurage_aquarium.renderer import Jellyfish, encode
        from kurage_aquarium.aquarium import Aquarium, DRIFTERS, PERIOD
        from kurage_aquarium.export import GlyphPainter

        fixtures = {"source": SOURCE, "python": sys.version.split()[0],
                    "numpy": np.__version__, "pillow": PIL.__version__,
                    "series": [], "poses": [], "points": [], "glyphs": [], "painters": [], "resizes": []}
        times = [-1/30, 0., 1/30, 2/30, 1.6, 6.4, 19.2, 38.4, 38.4+1/30]
        for scene in ("aquarium", "single", "looped_single"):
            for ascii_mode in ((False,) if scene == "aquarium" else (False, True)):
                for columns, rows in ((1, 1), (8, 5), (38, 22), (80, 8)):
                    model = Aquarium() if scene == "aquarium" else Jellyfish(
                        loop_period=19.2 if scene == "looped_single" else None)
                    series = {"scene": scene, "ascii": ascii_mode, "columns": columns,
                              "rows": rows, "temporal": True, "frames": []}
                    previous = None
                    for t in times:
                        frame = model.frame(columns, rows, t, temporal=True,
                                            **({} if scene == "aquarium" else {"mode": "ascii" if ascii_mode else "braille"}))
                        series["frames"].append({"time": t, "glyphs": "".join(frame[0].flat),
                                                 "colors": frame[1].reshape(-1, 3).tolist(),
                                                 "ansi": encode(frame), "ansi_delta": encode(frame, previous=previous)})
                        previous = frame
                    fixtures["series"].append(series)
        for scene in ("aquarium", "single", "looped_single"):
            for ascii_mode in ((False,) if scene=="aquarium" else (False,True)):
                model=Aquarium() if scene=="aquarium" else Jellyfish(loop_period=19.2 if scene=="looped_single" else None)
                case={"scene":scene,"ascii":ascii_mode,"frames":[]}
                for i,(w,h) in enumerate(((38,22),(76,11),(22,38),(44,19),(80,8),(40,16))):
                    t=i/30
                    frame=model.frame(w,h,t,temporal=True,**({} if scene=="aquarium" else {"mode":"ascii" if ascii_mode else "braille"}))
                    case["frames"].append({"time":t,"width":w,"height":h,"glyphs":"".join(frame[0].flat),"colors":frame[1].reshape(-1,3).tolist()})
                fixtures["resizes"].append(case)
        aquarium = Aquarium()
        for index in range(len(DRIFTERS)):
            for t in (-.01, 0., 1.6, 9.6, 19.2, 38.4):
                center, basis, velocity = aquarium.kinematics(index, t)
                fixtures["poses"].append({"index": index, "time": t, "center": center.tolist(),
                                          "basis": basis.tolist(), "velocity": velocity.tolist()})
        for detail in (1, 2):
            model = Jellyfish(detail=detail, loop_period=19.2)
            for t in (0., 1.6, 19.2):
                xyz, light = model.points(t, carry_swim=False)
                indices = sorted(set([0, len(xyz)-1, *np.linspace(0, len(xyz)-1, 80, dtype=int).tolist()]))
                fixtures["points"].append({"detail": detail, "time": t, "count": len(xyz),
                    "indices": indices, "xyz": xyz[indices].tolist(), "light": light[indices].tolist(),
                    "energy": light.sum(axis=0).tolist()})
        render = ROOT / "assets/render"
        render.mkdir(parents=True, exist_ok=True)
        for single in (False, True):
            name = "single" if single else "aquarium"
            cw, ch, fs, cols, rows, offset, period = ((10,20,14,120,50,40,19.2) if single else (8,16,12,160,45,0,38.4))
            font = ImageFont.truetype("DejaVuSans.ttf", fs)
            painter = GlyphPainter(font, cw, ch, offset_x=offset)
            (render / f"{name}-glyphs.bin").write_bytes(painter.atlas.astype("u1").tobytes())
            model = Jellyfish(loop_period=period) if single else Aquarium()
            samples = Image.new("RGB", (1280*3,720*2))
            for i in range(6):
                samples.paste(painter.draw(*model.frame(cols,rows,i*period/6)), (i%3*1280,i//3*720))
            (render / f"{name}-palette.bin").write_bytes(bytes(samples.quantize(colors=256).getpalette()))
            shader = model if single else model.shader
            for i in range(-18, 0): model.frame(cols, rows, i/30, temporal=True)
            for t in (0., 1/30, 2/30):
                frame = model.frame(cols,rows,t,temporal=True)
                fixtures["glyphs"].append({"single": single, "time": t,
                    "glyphs": "".join(frame[0].flat), "colors": frame[1].reshape(-1,3).tolist(),
                    "rgb_sha256": hashlib.sha256(painter.draw(*frame).tobytes()).hexdigest()})
        names = []
        for path in sorted((Path(temp)/"tests").glob("test_*.py")):
            names.extend(node.name for node in ast.walk(ast.parse(path.read_text()))
                         if isinstance(node, ast.FunctionDef) and node.name.startswith("test_"))
        fixtures["source_tests"] = sorted(names)
        from kurage_aquarium.renderer import BRAILLE
        glyphs = np.tile(BRAILLE.reshape(16,16), (1,2))
        colors = np.random.default_rng(0).integers(0,256,(*glyphs.shape,3),dtype=np.uint8)
        font_path = Path(ImageFont.truetype("DejaVuSans.ttf",12).path)
        (ROOT/"tests/fixtures/DejaVuSans.ttf").write_bytes(font_path.read_bytes())
        for size,cw,ch in ((12,8,16),(14,10,20),(18,8,16)):
            for offset in (-1,0,40):
                font = ImageFont.truetype(str(font_path),size)
                painter = GlyphPainter(font,cw,ch,offset_x=offset)
                fixtures["painters"].append({"size":size,"cell_width":cw,"cell_height":ch,"offset":offset,
                    "glyphs":"".join(glyphs.flat),"colors":colors.reshape(-1,3).tolist(),
                    "rgb_sha256":hashlib.sha256(painter.draw(glyphs,colors).tobytes()).hexdigest()})
        output = ROOT / "tests/fixtures/python-v0.3.0.json"
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(fixtures, ensure_ascii=False, separators=(",", ":")) + "\n")
        print(f"{len(fixtures['series'])} frame series, {len(fixtures['poses'])} poses, {len(names)} source tests")


if __name__ == "__main__":
    main()
