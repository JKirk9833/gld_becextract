use std::{
    fs::{create_dir_all, File},
    io::{Error, Read, Seek, SeekFrom, Write},
    path::Path,
};

fn read_word(f: &mut File, offset: u32) -> Result<u32, Error> {
    let mut buffer = vec![0; 4];

    f.seek(SeekFrom::Start(offset.into()))?;
    f.read(&mut buffer)?;

    let word = u32::from_be_bytes(buffer.try_into().unwrap());
    Ok(word)
}

fn read_word_le(f: &mut File, offset: u32) -> Result<u32, Error> {
    let mut buffer = vec![0; 4];

    f.seek(SeekFrom::Start(offset.into()))?;
    f.read(&mut buffer)?;

    let word = u32::from_le_bytes(buffer.try_into().unwrap());
    Ok(word)
}

fn read_bytes(f: &mut File, offset: u32, size: u32) -> Result<Vec<u8>, Error> {
    let mut buffer = vec![0; size.try_into().unwrap()];
    f.seek(SeekFrom::Start(offset.into()))?;
    f.read(&mut buffer)?;

    Ok(buffer)
}

// Why are we still here, just to buffer
fn read_byte(f: &mut File, offset: u32) -> Result<u8, Error> {
    let byte = read_bytes(f, offset, 0x1)?;

    Ok(byte[0])
}

fn read_short_le(f: &mut File, offset: u32) -> Result<u32, Error> {
    let mut buf = [0; 2];
    f.seek(SeekFrom::Start(offset as u64))?;
    f.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes([buf[0], buf[1], 0x0, 0x0]))
}

#[derive(Debug)]
#[allow(dead_code)]
struct FileEntry {
    path_hash: u32,
    data_offset: u32,
    correction: u32,
    unk: u8,
    data_size: u32,
    file_name: String,
}

fn read_file_entry(f: &mut File, offset: u32) -> Result<FileEntry, Error> {
    f.seek(SeekFrom::Start(offset as u64))?;

    let cor0 = read_byte(f, offset + 0x8)? as u32;
    let cor1 = read_byte(f, offset + 0x9)? as u32;
    let cor2 = read_byte(f, offset + 0xA)? as u32;

    let res: u32 = (cor2 << 16) + (cor1 << 8) + cor0;

    Ok(FileEntry {
        path_hash: read_word_le(f, offset)?,
        data_offset: read_word_le(f, offset + 0x4)?,
        correction: res,
        unk: read_byte(f, offset + 0xB)?,
        data_size: read_word_le(f, offset + 0xC)?,
        file_name: String::from("test.txt"),
    })
}

fn write_file(buffer: Vec<u8>, name: String) -> Result<(), Error> {
    let mut path = "./files/".to_owned();
    path.push_str(&name);

    let file = Path::new(&path);
    let directories = &file.parent().expect("Failed to get directories from path.");

    println!("Writing path: {}", directories.to_str().unwrap());

    create_dir_all(directories)?;

    let mut f = File::create(&file)?;
    f.write_all(&buffer)?;

    Ok(())
}

fn main() -> Result<(), Error> {
    let mut f = File::open("./files/gladius.bec")?;
    let file_alignment = read_short_le(&mut f, 0x6)?;
    let num_of_files = read_word_le(&mut f, 0x8)?;

    // Assuming I want to mess around with this later, I'll need to make this
    // filepath point to the newly generated hash file when recompiled
    let hash_file = File::open("./init_hashes.json")?;
    let file_hashes: serde_json::Value = serde_json::from_reader(hash_file)?;

    for i in 0..num_of_files {
        let fe_offset = (i + 1) * 0x10;
        let mut file_entry = read_file_entry(&mut f, fe_offset)?;
        let mut offset = file_entry.data_offset + file_entry.correction + (file_alignment - 1);

        if file_entry.correction > 0 {
            offset += 8;
        }
        offset = offset & (0x100000000 as u64 - file_alignment as u64) as u32;

        let hex_path = format!("0x{:x}", file_entry.path_hash);
        let name = file_hashes.get(hex_path);

        if name != None {
            // Set our file_entry file_name so we can keep track of what's what
            file_entry.file_name = name.unwrap().as_str().unwrap().to_string();
        } else {
            file_entry.file_name = format!("{}", i);
        }

        let file_bytes = read_bytes(&mut f, offset, file_entry.data_size)?;
        write_file(file_bytes, file_entry.file_name)?;
    }

    Ok(())
}
