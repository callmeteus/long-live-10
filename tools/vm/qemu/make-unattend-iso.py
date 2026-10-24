"""Builds the small ISO that Windows Setup reads for unattended installation."""

from pathlib import Path

import pycdlib


def main() -> None:
    root = Path(__file__).resolve().parent
    source = root / "autounattend.xml"
    output = root / "autounattend.iso"

    iso = pycdlib.PyCdlib()
    iso.new(interchange_level=3, joliet=3, rock_ridge="1.09", vol_ident="LL10UNATTEND")
    iso.add_file(
        str(source),
        iso_path="/AUTOUNATTEND.XML;1",
        joliet_path="/autounattend.xml",
        rr_name="autounattend.xml",
    )
    iso.write(str(output))
    iso.close()
    print(output)


if __name__ == "__main__":
    main()
