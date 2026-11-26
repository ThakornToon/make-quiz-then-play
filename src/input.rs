use std::io::{self, Write};

use crate::{
    model::{Question, MAX_CHOICES},
    AppResult,
};

pub fn read_question() -> AppResult<Option<Question>> {
    // อ่านข้อความจนกว่าจะพบ sentinel "." ซึ่งช่วยให้โจทย์มีหลายบรรทัดได้
    let prompt = loop {
        println!("กรอกโจทย์ (หลายบรรทัดได้ จบด้วยบรรทัดที่มี . เพียงตัวเดียว, พิมพ์ :q หรือ /cancel เพื่อยกเลิก)");
        let Some(prompt) = read_multiline()? else {
            return Ok(None);
        };
        if !prompt.trim().is_empty() {
            break prompt;
        }
        println!("โจทย์ต้องไม่ว่าง กรุณากรอกใหม่");
    };

    let Some(choice_count) = read_number_in_range_or_cancel(
        &format!("จำนวน choice (2 - {MAX_CHOICES}): "),
        2,
        MAX_CHOICES,
    )?
    else {
        return Ok(None);
    };
    let mut choices = Vec::new();
    choices.try_reserve_exact(choice_count)?;
    // เก็บ choice ตามลำดับที่ผู้ใช้กรอกไว้; answer จะอ้าง index ของลำดับนี้
    for index in 1..=choice_count {
        let Some(choice) = prompt_nonempty(&format!("Choice {index}: "))? else {
            return Ok(None);
        };
        choices.push(choice);
    }

    // อนุญาตให้สร้างชุดโจทย์ที่ยังไม่ใส่เฉลย เช่นไฟล์สำหรับแจกนักเรียน
    let Some(has_answer) = read_yes_no_or_cancel("ต้องการระบุเฉลยที่ถูกต้องหรือไม่? (y/n): ")?
    else {
        return Ok(None);
    };
    let answer = if has_answer {
        let Some(answer_num) =
            read_number_in_range_or_cancel("หมายเลข choice ที่ถูกต้อง: ", 1, choice_count)?
        else {
            return Ok(None);
        };
        Some(answer_num - 1)
    } else {
        None
    };
    let Some(has_explanation) =
        read_yes_no_or_cancel("ต้องการเพิ่มคำอธิบายเฉลยหรือไม่? (y/n): ")?
    else {
        return Ok(None);
    };
    let explanation = if has_explanation {
        println!("กรอกคำอธิบาย จบด้วยบรรทัดที่มี . เพียงตัวเดียว (พิมพ์ :q หรือ /cancel เพื่อยกเลิก)");
        let Some(explanation) = read_multiline()? else {
            return Ok(None);
        };
        // คำอธิบายว่างถือว่าไม่ได้ระบุ จึงบันทึกเป็น None
        (!explanation.trim().is_empty()).then_some(explanation)
    } else {
        None
    };
    let Some(shuffle_choices) =
        read_yes_no_or_cancel("สลับลำดับ choice ของข้อนี้หรือไม่? (y/n): ")?
    else {
        return Ok(None);
    };

    Ok(Some(Question {
        prompt,
        choices,
        answer,
        explanation,
        shuffle_choices,
    }))
}

/// อ่านข้อความหลายบรรทัดและถามซ้ำจนกว่าจะได้ข้อความที่ไม่ว่าง (คืน None เมื่อผู้ใช้ยกเลิก)
pub fn read_required_multiline(label: &str) -> AppResult<Option<String>> {
    loop {
        println!("กรอก{label} (จบด้วยบรรทัดที่มี . เพียงตัวเดียว, พิมพ์ :q หรือ /cancel เพื่อยกเลิก)");
        let Some(value) = read_multiline()? else {
            return Ok(None);
        };
        if !value.trim().is_empty() {
            return Ok(Some(value));
        }
        println!("{label}ต้องไม่ว่าง");
    }
}

/// ตรวจสอบว่าข้อความเป็นคำสั่งยกเลิก (:q หรือ /cancel) หรือไม่
pub fn is_cancel_command(input: &str) -> bool {
    matches!(input.trim().to_lowercase().as_str(), ":q" | "/cancel")
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

/// แสดง prompt อ่านหนึ่งบรรทัด โดยคืน None หากผู้ใช้พิมพ์ :q หรือ /cancel
pub fn prompt_or_cancel(message: &str) -> AppResult<Option<String>> {
    let input = prompt(message)?;
    if is_cancel_command(&input) {
        return Ok(None);
    }
    Ok(Some(input))
}

/// อ่านข้อความที่ห้ามว่าง และถามซ้ำหากผู้ใช้กรอกช่องว่างเท่านั้น (คืน None เมื่อผู้ใช้ยกเลิก)
pub fn prompt_nonempty(message: &str) -> AppResult<Option<String>> {
    loop {
        let input = prompt(message)?;
        if is_cancel_command(&input) {
            return Ok(None);
        }
        if !input.trim().is_empty() {
            return Ok(Some(input.trim().to_string()));
        }
        println!("ห้ามเว้นว่าง");
    }
}

/// อ่านข้อความหลายบรรทัด โดยใช้บรรทัด "." เป็นเครื่องหมายจบข้อมูล (คืน None เมื่อผู้ใช้ยกเลิก)
pub fn read_multiline() -> AppResult<Option<String>> {
    let mut lines = Vec::new();
    loop {
        let line = prompt("| ")?;
        if is_cancel_command(&line) {
            return Ok(None);
        }
        if line == "." {
            return Ok(Some(lines.join("\n")));
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

/// อ่านจำนวนเต็มในช่วง [minimum, maximum] โดยคืน None หากผู้ใช้พิมพ์ :q หรือ /cancel
pub fn read_number_in_range_or_cancel(
    message: &str,
    minimum: usize,
    maximum: usize,
) -> AppResult<Option<usize>> {
    loop {
        let input = prompt(message)?;
        if is_cancel_command(&input) {
            return Ok(None);
        }
        match input.trim().parse::<usize>() {
            Ok(number) if (minimum..=maximum).contains(&number) => return Ok(Some(number)),
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

/// อ่านคำตอบแบบใช่/ไม่ใช่ โดยคืน None หากผู้ใช้พิมพ์ :q หรือ /cancel
pub fn read_yes_no_or_cancel(message: &str) -> AppResult<Option<bool>> {
    loop {
        let input = prompt(message)?;
        if is_cancel_command(&input) {
            return Ok(None);
        }
        match input.trim().to_lowercase().as_str() {
            "y" | "yes" | "ใช่" => return Ok(Some(true)),
            "n" | "no" | "ไม่" => return Ok(Some(false)),
            _ => println!("กรุณาตอบ y หรือ n"),
        }
    }
}
