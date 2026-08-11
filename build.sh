#!/bin/bash
cargo +nightly build --release
python3 pack.py
