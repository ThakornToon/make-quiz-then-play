mod editor;
mod input;
mod model;
mod play;
mod storage;

use std::error::Error;

use crate::editor::{create_quiz, delete_quiz, edit_quiz, export_student_quiz};
use crate::input::prompt;
use crate::play::{grade_submission, take_quiz};

// รวม error จาก I/O, JSON และการตรวจข้อมูลให้ส่งกลับไปแสดงที่ main ได้ในรูปแบบเดียว
type AppResult<T> = Result<T, Box<dyn Error>>;

fn main() {
    if let Err(error) = run() {
        eprintln!("เกิดข้อผิดพลาด: {error}");
        std::process::exit(1);
    }
}

fn run() -> AppResult<()> {
    loop {
        println!("\n=== Quiz CLI ===");
        println!("1) เพิ่ม quiz");
        println!("2) ทำ quiz");
        println!("3) แก้ไข quiz");
        println!("4) ตรวจไฟล์คำตอบ");
        println!("5) ส่งออก quiz สำหรับนักเรียน (ไม่มีเฉลย)");
        println!("6) ลบ quiz");
        println!("0) ออกจากโปรแกรม");

        match prompt("เลือกเมนู: ")?.trim() {
            "1" => create_quiz()?,  // editor
            "2" => take_quiz()?,  // play
            "3" => edit_quiz()?,  // editor
            "4" => grade_submission()?,  // play
            "5" => export_student_quiz()?,  // editor
            "6" => delete_quiz()?,  // editor
            "0" => break,
            _ => println!("กรุณาเลือก 1, 2, 3, 4, 5, 6 หรือ 0"),
        }
    }
    Ok(())
}
