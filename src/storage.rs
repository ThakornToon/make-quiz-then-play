use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use crate::input::read_number_in_range;
use crate::{model::Quiz, AppResult};

pub const QUIZ_DIR: &str = "quizzes";
pub const RESULT_DIR: &str = "results";
pub const SUBMISSION_DIR: &str = "submissions";
pub const EXPORT_DIR: &str = "exports";

// โครงสร้างเก็บข้อมูล quiz ที่โหลดแล้วพร้อม path ของไฟล์ (ใช้อ่านไฟล์เพียงครั้งเดียว)
#[derive(Clone, Debug)]
pub struct LoadedQuiz {
    pub path: PathBuf,
    pub quiz: Quiz,
}

// โหลด quiz ทุกชุดจากโฟลเดอร์ quizzes เพียงครั้งเดียว
pub fn load_all_quizzes() -> AppResult<Vec<LoadedQuiz>> {
    let paths = quiz_files()?;
    let mut quizzes = Vec::with_capacity(paths.len());
    for path in paths {
        let file = File::open(&path)?;
        let quiz: Quiz = serde_json::from_reader(file)?;
        quizzes.push(LoadedQuiz { path, quiz });
    }
    Ok(quizzes)
}

// แสดงรายชื่อ quiz ทั้งหมด และให้ผู้ใช้เลือก 1 ชุด (อ่านไฟล์ครั้งเดียว)
pub fn select_loaded_quiz(prompt_text: &str) -> AppResult<Option<LoadedQuiz>> {
    let quizzes = load_all_quizzes()?;
    if quizzes.is_empty() {
        println!("ยังไม่มี quiz ในโฟลเดอร์ {QUIZ_DIR}/");
        return Ok(None);
    }
    println!("\nQuiz ที่มี:");
    for (index, item) in quizzes.iter().enumerate() {
        println!("{}. {} ({})", index + 1, item.quiz.title, item.path.display());
    }
    println!("0. ยกเลิก / กลับเมนูหลัก");
    let selected = read_number_in_range(prompt_text, 0, quizzes.len())?;
    if selected == 0 {
        println!("ยกเลิก กลับสู่เมนูหลัก");
        return Ok(None);
    }
    Ok(Some(quizzes.into_iter().nth(selected - 1).unwrap()))
}

// ลบไฟล์ quiz ออกจากโฟลเดอร์ quizzes
pub fn delete_quiz_file(path: &Path) -> AppResult<()> {
    fs::remove_file(path)?;
    Ok(())
}

// เขียนไฟล์ชั่วคราวให้เสร็จก่อนแทนที่ไฟล์เดิม เพื่อไม่ให้ JSON เดิมเสียหายระหว่างบันทึก
pub fn overwrite_quiz(path: &Path, quiz: &Quiz) -> AppResult<()> {
    let data = serde_json::to_vec_pretty(quiz)?;  // แปลง quiz เป็น JSON
    let directory = path.parent().unwrap_or_else(|| Path::new("."));  // หาโฟลเดอร์ของไฟล์ quiz
    let filename = path
        .file_name()
        .ok_or("ไม่สามารถระบุชื่อไฟล์ quiz ได้")?
        .to_string_lossy();  // แปลงเป็น String

    // สร้างไฟล์ชั่วคราวในโฟลเดอร์เดียวกันกับไฟล์ quiz เดิม
    for suffix in 0..1000 {
        // สร้างชื่อไฟล์ชั่วคราวที่ไม่ซ้ำกับไฟล์อื่นในโฟลเดอร์เดียวกัน
        let temporary_path =
            directory.join(format!(".{filename}.{}.{}.tmp", std::process::id(), suffix));
        // สร้างไฟล์ชั่วคราวและเขียนข้อมูล quiz ลงไป
        let mut temporary_file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        };

        // รัน closure แล้วเก็บผลลัพธ์
        let write_result = (|| -> io::Result<()> {
            temporary_file.write_all(&data)?;
            temporary_file.sync_all()?;  // ให้ระบบปฏิบัติการ sync ข้อมูลของไฟล์ชั่วคราวก่อนดำเนินการต่อ
            drop(temporary_file);  // ปิดไฟล์ชั่วคราวเพื่อให้สามารถ rename ได้
            fs::rename(&temporary_path, path)
        })();

        if let Err(error) = write_result {
            // หากเกิด error ระหว่างเขียนไฟล์ชั่วคราวหรือ rename ให้ลบไฟล์ชั่วคราวทิ้ง
            match fs::remove_file(&temporary_path) {
                Ok(()) => return Err(error.into()),
                Err(cleanup_error) if cleanup_error.kind() == io::ErrorKind::NotFound => {
                    return Err(error.into())  // ไฟล์ชั่วคราวถูกลบไปแล้ว ใช้ error เดิมเป็นผลลัพธ์
                }
                Err(cleanup_error) => {
                    return Err(format!(
                        "{error}; และลบไฟล์ชั่วคราว {} ไม่สำเร็จ: {cleanup_error}",
                        temporary_path.display()
                    )
                    .into())  // ลบไม่สำเร็จด้วยเหตุอื่น
                }
            }
        }
        return Ok(());
    }
    
    Err("ไม่สามารถสร้างไฟล์ชั่วคราวสำหรับบันทึก quiz ได้".into())
}

/// บันทึก quiz เป็น JSON แบบอ่านง่าย และไม่เขียนทับไฟล์เดิม
pub fn save_quiz(quiz: &Quiz) -> AppResult<PathBuf> {
    fs::create_dir_all(QUIZ_DIR)?;
    let path = unique_path(Path::new(QUIZ_DIR), &slug(&quiz.title), "json");
    let mut file = OpenOptions::new() // ตั้งค่าการเปิดไฟล์
        .write(true)  // อนุญาตให้เขียนข้อมูลลงไฟล์
        .create_new(true)  // สั่งให้สร้างไฟล์ใหม่ ถ้ามีไฟล์ชื่อนั้นอยู่แล้วจะคืน error ทันที
        .open(&path)?;  // เปิดหรือสร้างไฟล์เพื่อเขียนข้อมูล quiz
    serde_json::to_writer_pretty(&mut file, quiz)?;  // เขียนข้อมูล quiz เป็น JSON ลงไฟล์
    Ok(path)
}

/// บันทึก quiz ฉบับแจกนักเรียนโดยไม่เขียนทับไฟล์เฉลย
pub fn export_quiz(quiz: &Quiz) -> AppResult<PathBuf> {
    fs::create_dir_all(EXPORT_DIR)?;  // สร้างโฟลเดอร์ exports หากยังไม่มี
    let path = unique_path(
        Path::new(EXPORT_DIR),
        &format!("{}-student", slug(&quiz.title)),
        "json",
    ); 
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;  // สร้างไฟล์ใหม่โดยไม่เขียนทับไฟล์เดิม
    serde_json::to_writer_pretty(&mut file, quiz)?;
    Ok(path)
}

/// คืนรายชื่อไฟล์ JSON ในโฟลเดอร์ quiz
pub fn quiz_files() -> AppResult<Vec<PathBuf>> {
    json_files(Path::new(QUIZ_DIR))
}

/// คืนรายชื่อไฟล์คำตอบ JSON ที่รับมาจากผู้ทำ quiz
pub fn submission_files() -> AppResult<Vec<PathBuf>> {
    json_files(Path::new(SUBMISSION_DIR))
}

/// อ่านไฟล์ JSON จากโฟลเดอร์ที่ระบุ โดยคืนรายการว่างเมื่อโฟลเดอร์ยังไม่มี
fn json_files(directory: &Path) -> AppResult<Vec<PathBuf>> {
    // คืนรายการว่างเมื่อโฟลเดอร์ยังไม่มี
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();

    // อ่านไฟล์ JSON ตาม iterator ReadDir
    for entry in fs::read_dir(directory)? {
        // ใช้ ? จะตรวจ error ก่อนดึง DirEntry.path()
        let path = entry?.path();

        // ตรวจว่าเป็นไฟล์ JSON ไหม และเก็บ path ไว้
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// หา path ที่ยังไม่มีไฟล์ เพื่อรักษาไฟล์เดิมเมื่อชื่อ quiz ซ้ำกัน
pub fn unique_path(directory: &Path, base: &str, extension: &str) -> PathBuf {
    let mut suffix = 1;
    loop {
        let filename = if suffix == 1 {
            format!("{base}.{extension}")
        } else {
            format!("{base}-{suffix}.{extension}")
        };
        let candidate = directory.join(filename);  // ต่อ path 
        if !candidate.exists() {
            return candidate;  // คืน path ที่ยังไม่มีไฟล์
        }
        suffix += 1;
    }
}

/// ตรวจสอบว่าอักขระปลอดภัยสำหรับการใช้ในชื่อไฟล์ (รองรับ alphanumeric, ภาษาไทย และอักขระผสม)
fn is_safe_slug_char(character: char) -> bool {
    character.is_alphanumeric() || ('\u{0E01}'..='\u{0E5B}').contains(&character)
}

/// แปลงชื่อ quiz เป็นส่วนหนึ่งของชื่อไฟล์ โดยคงเฉพาะอักขระ alphanumeric และภาษาไทยที่ปลอดภัย
pub fn slug(value: &str) -> String {
    let slug = value
        .to_lowercase()
        .chars()
        .map(|character| {
            if is_safe_slug_char(character) {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        "quiz".to_string()
    } else {
        slug
    }
}
