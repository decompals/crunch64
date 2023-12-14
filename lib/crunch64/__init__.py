#!/usr/bin/env python3

from __future__ import annotations

# Version should be synced with lib/Cargo.toml and lib/pyproject.toml
__version_info__ = (0, 1, 0)
__version__ = ".".join(map(str, __version_info__))
__author__ = "Capybara man" # TODO: Ethan update this before release plz

from . import yay0 as yay0
from . import yaz0 as yaz0
