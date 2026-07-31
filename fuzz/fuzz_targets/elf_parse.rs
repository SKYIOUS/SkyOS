#![no_main]

use libfuzzer_sys::fuzz_target;
use xmas_elf::ElfFile;

fuzz_target!(|data: &[u8]| {
    if let Ok(elf) = ElfFile::new(data) {
        let _ = elf.header;
        for ph in elf.program_iter() {
            let _ = ph.get_type();
            let _ = ph.flags();
            let _ = ph.physical_addr();
            let _ = ph.virtual_addr();
            let _ = ph.file_size();
            let _ = ph.mem_size();
            let _ = ph.align();
        }
        for sh in elf.section_iter() {
            let _ = sh.get_type();
            let _ = sh.flags();
            let _ = sh.address();
            let _ = sh.size();
        }
    }
});
