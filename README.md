# ferrous_gb

`ferrous_gb` is a Gameboy emulator written in Rust targeting the web (through WASM) and native platforms.

## To Do

- [ ] Finish APU/sound
- [ ] More debugging tools

## Accuracy

Blargg Test Results:
- `cpu_instrs`: PASS
- `instr_timing`: PASS
- `dmg_sound`: 4/12

## Credits

- [Bootix](https://github.com/Hacktix/Bootix), a CC0 bootrom replacement

Some open-source game(s) have been included as examples. Try them with `File -> Load Example` in the menu bar.
- [Tobu Tobu Girl](https://github.com/SimonLarsen/tobutobugirl) (MIT/CC-BY 4.0, © 2017 Tangram Games)

