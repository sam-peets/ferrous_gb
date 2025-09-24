# ferrous_gb

`ferrous_gb` is a WIP Gameboy emulator built in Rust targeting the web (through WASM) and native platforms.

https://github.com/user-attachments/assets/7a8cb2ab-2506-469f-9942-42bf58288b1e

## To Do

- [x] APU/sound
- [ ] Polish APU/sound
- [ ] Use M-cycle accurate memory reads on the CPU
- [ ] More debugging tools

## Accuracy

Blargg Test Results:
- `cpu_instrs`: PASS
- `instr_timing`: PASS
- `dmg_sound`
    - `01-registers`: PASS
    - `02-len ctr`: PASS
    - `03-trigger`: PASS
    - `04-sweep`: PASS
    - `05-sweep details`: PASS
    - `06-overflow on trigger`: PASS
    - `07-len sweep period sync`: PASS
    - `08-len ctr during power`: PASS
    - `09-wave read while on`: 1/?
    - `10-wave trigger while on`: 1/?
    - `11-regs after power`: PASS
    - `12-wave write while on`: 1/?

## Credits

- [Bootix](https://github.com/Hacktix/Bootix), a CC0 bootrom replacement

Some open-source game(s) have been included as examples. Try them with `File -> Load Example` in the menu bar.
- [Tobu Tobu Girl](https://github.com/SimonLarsen/tobutobugirl) (MIT/CC-BY 4.0, © 2017 Tangram Games)

