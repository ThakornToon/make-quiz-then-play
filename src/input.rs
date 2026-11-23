use std::io::{self, Write};

use crate::{
    model::{Question, MAX_CHOICES},
    AppResult,
};

pub fn read_question() -> AppResult<Question> {
    // อ่านข้อความจนกว่าจะพบ sentinel "." ซึ่งช่วยให้โจทย์มีหลายบรรทัดได้
    let prompt = loop {
        println!("กรอกโจทย์ (หลายบรรทัดได้ จบด้วยบรรทัดที่มี . เพียงตัวเดียว)");
        let prompt = read_multiline()?;
        if !prompt.trim().is_empty() {
            break prompt;
        }
        println!("โจทย์ต้องไม่ว่าง กรุณากรอกใหม่");
    };

    let choice_count = read_number_in_range(
        &format!("จำนวน choice (2 - {MAX_CHOICES}): "),
        2,
        MAX_CHOICES,
    )?;
    let mut choices = Vec::new();
    choices.try_reserve_exact(choice_count)?;
    // เก็บ choice ตามลำดับที่ผู้ใช้กรอกไว้; answer จะอ้าง index ของลำดับนี้
    for index in 1..=choice_count {
        choices.push(prompt_nonempty(&format!("Choice {index}: "))?);
    }

    // อนุญาตให้สร้างชุดโจทย์ที่ยังไม่ใส่เฉลย เช่นไฟล์สำหรับแจกนักเรียน
    let answer = if read_yes_no("ต้องการระบุเฉลยที่ถูกต้องหรือไม่? (y/n): ")?
    {
        Some(read_number_in_range("หมายเลข choice ที่ถูกต้อง: ", 1, choice_count)? - 1)
    } else {
        None
    };
    let explanation = if read_yes_no("ต้องการเพิ่มคำอธิบายเฉลยหรือไม่? (y/n): ")?
    {
        println!("กรอกคำอธิบาย จบด้วยบรรทัดที่มี . เพียงตัวเดียว");
        let explanation = read_multiline()?;
        // คำอธิบายว่างถือว่าไม่ได้ระบุ จึงบันทึกเป็น None
        (!explanation.trim().is_empty()).then_some(explanation)
    } else {
        None
    };
    let shuffle_choices = read_yes_no("สลับลำดับ choice ของข้อนี้หรือไม่? (y/n): ")?;

    Ok(Question {
        prompt,
        choices,
        answer,
        explanation,
        shuffle_choices,
    })
}

/// อ่านข้อความหลายบรรทัดและถามซ้ำจนกว่าจะได้ข้อความที่ไม่ว่าง
pub fn read_required_multiline(label: &str) -> AppResult<String> {
    loop {
        println!("กรอก{label} (จบด้วยบรรทัดที่มี . เพียงตัวเดียว)");
        let value = read_multiline()?;
        if !value.trim().is_empty() {
            return Ok(value);
        }
        println!("{label}ต้องไม่ว่าง");
    }
}

/// แสดง prompt อ่านหนึ่งบรรทัด และคืน error เมื่อ input ถูกปิดก่อนเวลา
pub fn prompt(message: &str) -> AppResult<String> {
    print!("{message}");
    io::stdout().flush()?;
    let mut input = String::new();
    if io::stdin().read_line(&mut input)? == 0 {
        return Err("ปิด input ก่อนจบการทำงาน".into());
    }
    Ok(input.trim_end_matches(['\r', '\n']).to_string())
}

/// อ่านข้อความที่ห้ามว่าง และถามซ้ำหากผู้ใช้กรอกช่องว่างเท่านั้น
pub fn prompt_nonempty(message: &str) -> AppResult<String> {
    loop {
        let input = prompt(message)?;
        if !input.trim().is_empty() {
            return Ok(input.trim().to_string());
        }
        println!("ห้ามเว้นว่าง");
    }
}

/// อ่านข้อความหลายบรรทัด โดยใช้บรรทัด "." เป็นเครื่องหมายจบข้อมูล
pub fn read_multiline() -> AppResult<String> {
    let mut lines = Vec::new();
    loop {
        let line = prompt("| ")?;
        if line == "." {
            return Ok(lines.join("\n"));
        }
        lines.push(line);
    }
}

/// อ่านจำนวนเต็มที่อยู่ในช่วงปิด [minimum, maximum] และถามซ้ำเมื่ออยู่นอกช่วง
pub fn read_number_in_range(
    message: &str,
    minimum: usize,
    maximum: usize,
) -> AppResult<usize> {
    loop {
        let input = prompt(message)?;
        match input.trim().parse::<usize>() {
            Ok(number) if (minimum..=maximum).contains(&number) => return Ok(number),
            _ => println!("กรุณากรอกจำนวนเต็มตั้งแต่ {minimum} ถึง {maximum}"),
        }
    }
}

/// อ่านคำตอบแบบใช่/ไม่ใช่ รองรับทั้ง y/n ภาษาอังกฤษและคำตอบภาษาไทย
pub fn read_yes_no(message: &str) -> AppResult<bool> {
    loop {
        match prompt(message)?.trim().to_lowercase().as_str() {
            "y" | "yes" | "ใช่" => return Ok(true),
            "n" | "no" | "ไม่" => return Ok(false),
            _ => println!("กรุณาตอบ y หรือ n"),
        }
    }
}
