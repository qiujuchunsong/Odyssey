/*从rCore复制来的 */
use std::fs::{File, read_dir};
use std::io::{Result, Write};

fn main() {
    println!("cargo:rerun-if-changed=../user/src/");
    println!("cargo:rerun-if-changed={}", TARGET_PATH);
    insert_app_data().unwrap();
}

static TARGET_PATH: &str = "../user/target/riscv64gc-unknown-none-elf/release/";

fn insert_app_data() -> Result<()> {
    let mut f = File::create("src/link_app.S").unwrap();
    let mut apps: Vec<_> = read_dir("../user/src/bin")
        .unwrap()
        .into_iter()
        .map(|dir_entry| {
            let mut name_with_ext = dir_entry.unwrap().file_name().into_string().unwrap();
            name_with_ext.drain(name_with_ext.find('.').unwrap()..name_with_ext.len());
            name_with_ext
        })
        .collect();
    apps.sort();

    writeln!(
        f,
        r#"
    .align 3
    .section .data
    .global _num_app
_num_app:
    .quad {}"#,
        apps.len()
    )?;

    for i in 0..apps.len() {
        writeln!(f, r#"    .quad app_{}_start"#, i)?;
    }
    writeln!(f, r#"    .quad app_{}_end"#, apps.len() - 1)?;

    writeln!(f, r#"
.global _app_names
_app_names:"#)?;
for app in apps.iter() {
    writeln!(f, r#"    .string "{}""#, app)?;
}

    for (idx, app) in apps.iter().enumerate() {
        println!("app_{}: {}", idx, app);
        // 【ch4 变化】这里内嵌的必须是【ELF 文件本身】，而不是 ch3 那种用
        // objcopy 出来的裸二进制 <app>.bin。
        //   原因：ch4 的 MemorySet::from_elf() 要用 xmas-elf 解析 program header
        //        （virtual_addr / mem_size / flags / offset ...），裸二进制没有任何
        //        头部信息，一解析就报 "Did not find ELF magic number"。
        //   ch3 之所以用 .bin：那时内核只是把字节整块拷到固定物理地址，不需要元信息。
        writeln!(
            f,
            r#"
    .section .data
    .global app_{0}_start
    .global app_{0}_end
app_{0}_start:
    .incbin "{2}{1}"
app_{0}_end:"#,
            idx, app, TARGET_PATH
        )?;
    }
    Ok(())
}
