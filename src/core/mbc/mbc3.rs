use crate::core::mbc::Mbc;

#[derive(Debug)]
pub struct Mbc3 {
    rom: Vec<Vec<u8>>,
    ram: Vec<Vec<u8>>,
    rom_banks: usize,
    ram_banks: usize,
    battery: bool,
    rom_bank: u8,
    ram_bank: u8,
    ram_enable: bool,
}

impl Mbc3 {
    pub fn new(rom: &[u8], rom_banks: usize, ram_banks: usize, battery: bool, timer: bool) -> Self {
        let rom = rom.chunks(0x4000).map(<[u8]>::to_vec).collect();
        let ram = vec![vec![0; 0x2000]; ram_banks];
        Self {
            rom,
            ram,
            rom_banks,
            ram_banks,
            battery,
            rom_bank: 1,
            ram_bank: 0,
            ram_enable: false,
        }
    }
}

impl Mbc for Mbc3 {
    fn read(&self, addr: u16) -> u8 {
        let addr = addr as usize;
        let rom_bank = self.rom_bank as usize;
        let ram_bank = self.ram_bank as usize;
        match addr {
            0x0000..=0x3fff => self.rom[0][addr],
            0x4000..=0x7fff => self.rom[rom_bank][addr - 0x4000],
            0xa000..=0xbfff => {
                if self.ram_enable {
                    match self.ram_bank {
                        0x0..=0x07 => {
                            // ram banks
                            self.ram[ram_bank][addr - 0xa000]
                        }
                        0x08..=0x0c => {
                            // TODO: rtc banks
                            0xff
                        }
                        _ => {
                            log::warn!("Mbc3: bad ram bank: 0x{ram_bank:02x?}");
                            0xff
                        }
                    }
                } else {
                    0xff
                }
            }
            _ => {
                log::warn!("Mbc3: invalid read: 0x{addr:04x?}");
                0xff
            }
        }
    }

    fn write(&mut self, addr: u16, val: u8) {
        let addr = addr as usize;
        let ram_bank = self.ram_bank as usize;
        match addr {
            0x0000..=0x1fff => {
                self.ram_enable = (val & 0x0f) == 0x0a;
            }
            0x2000..=0x3fff => {
                self.rom_bank = if val == 0 { 0x01 } else { val & 0b0111_1111 };
            }
            0x4000..=0x5fff => {
                self.ram_bank = val;
            }
            0x6000..=0x7fff => {
                // TODO: latch clock data
            }
            0xa000..=0xbfff => {
                if self.ram_enable {
                    match self.ram_bank {
                        0x0..=0x07 => {
                            // ram banks
                            self.ram[ram_bank][addr - 0xa000] = val;
                        }
                        0x08..=0x0c => {
                            // TODO: rtc banks
                        }
                        _ => {
                            log::warn!("Mbc3: bad ram bank: 0x{ram_bank:02x?}");
                        }
                    }
                }
            }
            _ => log::warn!("Mbc3: invalid write: 0x{addr:04x?}"),
        }
    }
}
