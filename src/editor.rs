use crate::input::{
    prompt, prompt_nonempty, read_multiline, read_number_in_range, read_question,
    read_required_multiline, read_yes_no,
};
use crate::model::{
    remove_answer_key, validate_quiz, Question, QuestionLocation, Quiz, QuizBlock, MAX_CHOICES,
};
use crate::storage::{
    delete_quiz_file, export_quiz, overwrite_quiz, save_quiz, select_loaded_quiz, LoadedQuiz,
};
use crate::AppResult;

pub fn create_quiz() -> AppResult<()> {
    // สร้าง quiz ใหม่จากชื่อและบล็อกโจทย์ที่ผู้ใช้เพิ่มเข้ามาทีละรายการ
    let title = prompt_nonempty("ชื่อ quiz: ")?;
    let mut blocks = Vec::new();

    loop {
        println!("\nเพิ่มเนื้อหา: [q] โจทย์เดี่ยว  [g] กลุ่ม  [done] บันทึก  [cancel] ยกเลิก");
        match prompt("เลือก: ")?.trim().to_lowercase().as_str() {
            // โจทย์เดี่ยวจะกลายเป็นหนึ่งบล็อกที่สลับตำแหน่งได้
            "q" => blocks.push(QuizBlock::Question {
                question: read_question()?,
            }),
            "g" => {
                // ข้อมูลกลุ่มถูกเก็บรวมเป็นบล็อกเดียว เพื่อรักษาความสัมพันธ์และลำดับโจทย์
                let group_title = prompt_nonempty("ชื่อกลุ่ม: ")?;
                println!("กรอกบทความ/เนื้อหากลุ่ม จบด้วยบรรทัดที่มี . เพียงตัวเดียว");
                let passage = read_multiline()?;
                let count = read_number_in_range("จำนวนโจทย์ในกลุ่ม (1 - 100): ", 1, 100)?;
                let mut questions = Vec::new();
                questions.try_reserve_exact(count)?;
                // อ่านโจทย์ย่อยตามลำดับ โดยยังไม่สลับลำดับภายในกลุ่ม
                for index in 1..=count {
                    println!("\nโจทย์ที่ {index}/{count}");
                    questions.push(read_question()?);
                }
                blocks.push(QuizBlock::Group {
                    title: group_title,
                    passage,
                    questions,
                });
            }
            // ป้องกันไม่ให้บันทึก quiz ที่ไม่มีโจทย์
            "done" if !blocks.is_empty() => break,
            "done" => println!("ต้องเพิ่มโจทย์อย่างน้อยหนึ่งข้อ"),
            "cancel" | "x" => {
                if blocks.is_empty()
                    || read_yes_no(
                        "มีข้อมูลที่กรอกไว้แล้ว ยืนยันยกเลิกโดยไม่บันทึกหรือไม่? (y/n): ",
                    )?
                {
                    println!("ยกเลิกการสร้าง quiz");
                    return Ok(());
                }
            }
            _ => println!("กรุณาเลือก q, g, done หรือ cancel"),
        }
    }

    // เขียนข้อมูลทั้งหมดหลังจากผู้ใช้ระบุว่าจบการเพิ่มโจทย์
    let quiz = Quiz { title, blocks };
    let path = save_quiz(&quiz)?;
    println!("บันทึก quiz แล้ว: {}", path.display());
    Ok(())
}

/// สร้างสำเนาสำหรับนักเรียนโดยลบเฉลยและคำอธิบาย แล้วบันทึกแยกจากไฟล์ต้นฉบับ
pub fn export_student_quiz() -> AppResult<()> {
    let Some(LoadedQuiz { mut quiz, .. }) = select_loaded_quiz("เลือก quiz ที่ต้องการส่งออก: ")? else {
        return Ok(());
    };
    validate_quiz(&quiz)?;
    remove_answer_key(&mut quiz);
    let path = export_quiz(&quiz)?;
    println!("ส่งออก quiz ฉบับไม่มีเฉลยแล้ว: {}", path.display());
    println!("คัดลอกไฟล์นี้ไปไว้ในโฟลเดอร์ quizzes/ ของเครื่องนักเรียน");
    Ok(())
}

/// เลือก quiz และแก้ไขข้อมูลในหน่วยความจำก่อนถามยืนยันเพื่อบันทึกทับไฟล์เดิม
pub fn edit_quiz() -> AppResult<()> {
    let Some(LoadedQuiz { path, mut quiz }) =
        select_loaded_quiz("เลือก quiz ที่ต้องการแก้ไข: ")?
    else {
        return Ok(());
    };
    validate_quiz(&quiz)?;
    let original = quiz.clone();

    loop {
        println!("\nแก้ไข quiz: {}", quiz.title);
        println!("1) เปลี่ยนชื่อ quiz");
        println!("2) แก้ไขโจทย์");
        println!("3) เพิ่มโจทย์");
        println!("4) ลบข้อ (ลบโจทย์)");
        println!("5) ย้ายโจทย์ (เข้า/ออก/ข้ามกลุ่ม)");
        println!("6) ย้ายทั้งกลุ่มหรือโจทย์เดี่ยว");
        println!("7) แก้ไขชื่อกลุ่ม/บทความ");
        println!("0) เสร็จและกลับ");
        match prompt("เลือก: ")?.trim() {
            "1" => quiz.title = prompt_nonempty("ชื่อ quiz ใหม่: ")?,
            "2" => edit_question(&mut quiz)?,
            "3" => add_question(&mut quiz)?,
            "4" => delete_question(&mut quiz)?,
            "5" => move_question(&mut quiz)?,
            "6" => move_block(&mut quiz)?,
            "7" => edit_group(&mut quiz)?,
            "0" => break,
            _ => println!("กรุณาเลือกเมนู 0 ถึง 7"),
        }
    }

    if quiz == original {
        println!("ไม่มีการเปลี่ยนแปลง");
    } else if read_yes_no("ยืนยันบันทึกทับไฟล์ quiz เดิมหรือไม่? (y/n): ")? {
        validate_quiz(&quiz)?;
        overwrite_quiz(&path, &quiz)?;
        println!("บันทึกการแก้ไขแล้ว: {}", path.display());
    } else {
        println!("ยกเลิกการแก้ไข ไฟล์เดิมไม่เปลี่ยนแปลง");
    }
    Ok(())
}

/// ลบไฟล์ quiz ที่เลือกหลังยืนยัน
pub fn delete_quiz() -> AppResult<()> {
    let Some(LoadedQuiz { path, quiz }) = select_loaded_quiz("เลือก quiz ที่ต้องการลบ: ")? 
    else {
        return Ok(());
    };
    if read_yes_no(&format!("ยืนยันลบ quiz \"{}\" ({}) หรือไม่? (y/n): ", quiz.title, path.display()))? {
        delete_quiz_file(&path)?;
        println!("ลบ quiz \"{}\" เรียบร้อยแล้ว", quiz.title);
    } else {
        println!("ยกเลิกการลบ quiz ไฟล์เดิมไม่เปลี่ยนแปลง");
    }
    Ok(())
}

/// แสดงโจทย์พร้อมตำแหน่งปัจจุบัน แล้วคืนตำแหน่งที่ผู้ใช้เลือกในโครงสร้างข้อมูล
fn select_question_location(quiz: &Quiz) -> AppResult<Option<QuestionLocation>> {
    let locations = quiz.question_locations();
    if locations.is_empty() {
        return Err("quiz นี้ไม่มีโจทย์".into());
    }
    display_questions(quiz, &locations);
    println!("0. ยกเลิก");
    let selected = read_number_in_range("เลือกหมายเลขโจทย์: ", 0, locations.len())?;
    if selected == 0 {
        return Ok(None);
    }
    Ok(Some(locations[selected - 1]))
}

/// แสดงโจทย์ทุกข้อแบบแบนพร้อมชื่อกลุ่ม เพื่อให้เลือกได้ทั้งโจทย์เดี่ยวและโจทย์ย่อย
fn display_questions(quiz: &Quiz, locations: &[QuestionLocation]) {
    println!("\nโจทย์ใน quiz:");
    for (index, location) in locations.iter().enumerate() {
        let block = &quiz.blocks[location.block_index];
        let question = quiz
            .question_at(*location)
            .expect("ตำแหน่งโจทย์ต้องมาจาก question_locations");
        match block {
            QuizBlock::Question { .. } => {
                println!(
                    "{}. [เดี่ยว] {}",
                    index + 1,
                    question.prompt.lines().next().unwrap_or("")  // แสดงบรรทัดแรกของ prompt
                );
            }
            QuizBlock::Group { title, .. } => println!(
                "{}. [กลุ่ม: {title}] {}",
                index + 1,
                question.prompt.lines().next().unwrap_or("")  // แสดงบรรทัดแรกของ prompt
            ),
        }
    }
}

/// แก้ prompt, choice/เฉลย, คำอธิบาย หรือการสลับ choice ของโจทย์ที่เลือก
fn edit_question(quiz: &mut Quiz) -> AppResult<()> {
    let Some(location) = select_question_location(quiz)? else {
        println!("ยกเลิก");
        return Ok(());
    };
    loop {
        println!("\nแก้โจทย์:");
        println!("1) ข้อความโจทย์");
        println!("2) ตัวเลือกและเฉลย");
        println!("3) คำอธิบายเฉลย");
        println!("4) สลับลำดับ choice");
        println!("0) กลับ");
        match prompt("เลือก: ")?.trim() {
            "1" => {
                quiz.question_at_mut(location)
                    .expect("ตำแหน่งโจทย์ต้องมาจาก question_locations")
                    .prompt = read_required_multiline("ข้อความโจทย์")?;
            }
            "2" => edit_choices(
                quiz.question_at_mut(location)
                    .expect("ตำแหน่งโจทย์ต้องมาจาก question_locations"),
            )?,
            "3" => edit_explanation(
                quiz.question_at_mut(location)
                    .expect("ตำแหน่งโจทย์ต้องมาจาก question_locations"),
            )?,
            "4" => {
                let question = quiz
                    .question_at_mut(location)
                    .expect("ตำแหน่งโจทย์ต้องมาจาก question_locations");
                question.shuffle_choices = !question.shuffle_choices;
                println!(
                    "สลับ choice: {}",
                    if question.shuffle_choices {
                        "เปิด"
                    } else {
                        "ปิด"
                    }
                );
            }
            "0" => break,
            _ => println!("กรุณาเลือกเมนู 0 ถึง 4"),
        }
    }
    Ok(())
}

/// แก้ข้อความ choice เพิ่ม/ลบ choice หรือเปลี่ยนคำตอบที่ถูกต้อง
fn edit_choices(question: &mut Question) -> AppResult<()> {
    loop {
        println!("\nตัวเลือกปัจจุบัน:");
        for (index, choice) in question.choices.iter().enumerate() {
            let marker = if question.answer == Some(index) {
                " (เฉลย)"
            } else {
                ""
            };
            println!("{}. {}{marker}", index + 1, choice);
        }
        println!("1) แก้ข้อความ choice");
        println!("2) เพิ่ม choice");
        println!("3) ลบ choice");
        println!("4) กำหนด/เปลี่ยน choice ที่เป็นเฉลย");
        println!("5) ลบเฉลย");
        println!("0) กลับ");
        match prompt("เลือก: ")?.trim() {
            "1" => {
                let index = read_number_in_range("เลือก choice ที่จะแก้: ", 1, question.choices.len())?;
                question.choices[index - 1] = prompt_nonempty("ข้อความ choice ใหม่: ")?;
            }
            "2" if question.choices.len() < MAX_CHOICES => {
                question
                    .choices
                    .push(prompt_nonempty("ข้อความ choice ใหม่: ")?);
            }
            "2" => println!("เพิ่ม choice ได้ไม่เกิน {MAX_CHOICES} ตัวเลือก"),
            "3" if question.choices.len() > 2 => {
                let index =
                    read_number_in_range("เลือก choice ที่จะลบ: ", 1, question.choices.len())? - 1;
                if !read_yes_no(&format!(
                    "ยืนยันลบ choice {} ({}) หรือไม่? (y/n): ",
                    index + 1,
                    question.choices[index]
                ))? {
                    continue;
                }

                question.choices.remove(index);

                if question.answer == Some(index) {
                    println!("ตัวเลือกที่เหลือ:");
                    for (choice_index, choice) in question.choices.iter().enumerate() {
                        println!("{}. {}", choice_index + 1, choice);
                    }
                    question.answer = if read_yes_no("ต้องการกำหนดเฉลยใหม่หรือไม่? (y/n): ")?
                    {
                        Some(
                            read_number_in_range(
                                "เลือกหมายเลขเฉลยใหม่: ",
                                1,
                                question.choices.len(),
                            )? - 1,
                        )
                    } else {
                        None
                    };
                } else if question.answer.is_some_and(|answer| answer > index) {
                    question.answer = question.answer.map(|answer| answer - 1);
                }
            }
            "3" => println!("ต้องเหลือ choice อย่างน้อย 2 ตัวเลือก"),
            "4" => {
                question.answer = Some(
                    read_number_in_range("หมายเลข choice ที่ถูกต้อง: ", 1, question.choices.len())? - 1,
                );
            }
            "5" => question.answer = None,
            "0" => break,
            _ => println!("กรุณาเลือกเมนู 0 ถึง 5"),
        }
    }
    Ok(())
}

/// แก้คำอธิบายเฉลย หรือเลือกลบคำอธิบายเดิม
fn edit_explanation(question: &mut Question) -> AppResult<()> {
    println!(
        "คำอธิบายปัจจุบัน: {}",
        question.explanation.as_deref().unwrap_or("(ไม่มี)")
    );
    println!("1) เขียนคำอธิบายใหม่");
    println!("2) ลบคำอธิบาย");
    println!("0) กลับ");
    match prompt("เลือก: ")?.trim() {
        "1" => {
            println!("กรอกคำอธิบายใหม่ จบด้วยบรรทัดที่มี . เพียงตัวเดียว");
            let explanation = read_multiline()?;
            question.explanation = (!explanation.trim().is_empty()).then_some(explanation);
        }
        "2" => question.explanation = None,
        "0" => {}
        _ => println!("ไม่มีการเปลี่ยนแปลง"),
    }
    Ok(())
}

/// เพิ่มโจทย์เดี่ยว เพิ่มโจทย์ท้ายกลุ่มที่มีอยู่ หรือสร้างกลุ่มใหม่
fn add_question(quiz: &mut Quiz) -> AppResult<()> {
    println!("เพิ่มโจทย์แบบใด?");
    println!("1) เพิ่มเป็นโจทย์เดี่ยวท้าย quiz");
    println!("2) เพิ่มท้ายกลุ่มที่มีอยู่");
    println!("3) สร้างกลุ่มใหม่พร้อมโจทย์");
    println!("0) ยกเลิก");
    match prompt("เลือก: ")?.trim() {
        "0" => return Ok(()),
        "1" => quiz.blocks.push(QuizBlock::Question {
            question: read_question()?,
        }),
        "2" => {
            let groups = group_block_indices(quiz);
            if groups.is_empty() {
                println!("ยังไม่มีกลุ่มให้เพิ่มโจทย์");
                return Ok(());
            }
            let Some(block_index) = select_group(quiz, &groups)? else {
                println!("ยกเลิก");
                return Ok(());
            };
            let question = read_question()?;
            if let QuizBlock::Group { questions, .. } = &mut quiz.blocks[block_index] {
                questions.push(question);
            }
        }
        "3" => {
            let title = prompt_nonempty("ชื่อกลุ่ม: ")?;
            println!("กรอกบทความ/เนื้อหากลุ่ม จบด้วยบรรทัดที่มี . เพียงตัวเดียว");
            let passage = read_multiline()?;
            let question = read_question()?;
            quiz.blocks.push(QuizBlock::Group {
                title,
                passage,
                questions: vec![question],
            });
        }
        _ => println!("ไม่มีการเพิ่มโจทย์"),
    }
    Ok(())
}

/// ลบโจทย์ที่เลือกหลังยืนยัน; กลุ่มจะถูกลบอัตโนมัติเมื่อไม่มีโจทย์เหลือ
fn delete_question(quiz: &mut Quiz) -> AppResult<()> {
    let locations = quiz.question_locations();
    if locations.len() == 1 {
        println!("ต้องเหลือโจทย์อย่างน้อยหนึ่งข้อ จึงไม่สามารถลบข้อนี้ได้");
        return Ok(());
    }
    display_questions(quiz, &locations);
    println!("0. ยกเลิก");
    let selected = read_number_in_range("เลือกหมายเลขโจทย์: ", 0, locations.len())?;
    if selected == 0 {
        println!("ยกเลิก");
        return Ok(());
    }
    let location = locations[selected - 1];
    let question = quiz
        .question_at(location)
        .expect("ตำแหน่งโจทย์ต้องมาจาก question_locations");
    let summary = question.prompt.lines().next().unwrap_or("");
    if !read_yes_no(&format!("ยืนยันลบโจทย์ \"{summary}\" หรือไม่? (y/n): "))?
    {
        println!("ยกเลิกการลบ");
        return Ok(());
    }
    let removed_group = match (&mut quiz.blocks[location.block_index], location.question_index,) {
        (QuizBlock::Question { .. }, None) => {
            quiz.blocks.remove(location.block_index);
            false
        }
        (QuizBlock::Group { questions, .. }, Some(question_index)) => {
            questions.remove(question_index);
            if questions.is_empty() {
                quiz.blocks.remove(location.block_index);
                true
            } else {
                false
            }
        }
        _ => unreachable!("ตำแหน่งโจทย์ต้องตรงกับชนิดบล็อก"),
    };
    if removed_group {
        println!("ลบโจทย์แล้ว กลุ่มถูกนำออกด้วยเนื่องจากไม่มีโจทย์เหลือ");
    } else {
        println!("ลบโจทย์แล้ว");
    }
    Ok(())
}

/// ย้ายโจทย์เดี่ยว/โจทย์ในกลุ่มไปยังตำแหน่งใหม่และเลือกรูปแบบปลายทางได้
fn move_question(quiz: &mut Quiz) -> AppResult<()> {
    let Some(source) = select_question_location(quiz)? else {
        println!("ยกเลิก");
        return Ok(());
    };
    let question = take_question(quiz, source);
    let groups = group_block_indices(quiz);
    println!("ย้ายโจทย์ไปที่ใด?");
    println!("0) ยกเลิก (โจทย์จะถูกเพิ่มกลับเป็นโจทย์เดี่ยวท้าย quiz)");
    println!("1) เป็นโจทย์เดี่ยว");
    if !groups.is_empty() {
        println!("2) เข้าไปในกลุ่ม");
    }
    let destination =
        read_number_in_range("เลือกปลายทาง: ", 0, if groups.is_empty() { 1 } else { 2 })?;
    if destination == 0 {
        quiz.blocks.push(QuizBlock::Question { question });
        println!("ยกเลิกการย้าย โจทย์ถูกเพิ่มกลับเป็นโจทย์เดี่ยวท้าย quiz");
        return Ok(());
    }
    match destination {
        1 => {
            display_blocks(quiz);
            let position = read_number_in_range(
                "แทรกก่อนบล็อกหมายเลขใด (หรือเลือกหลังสุด): ",
                1,
                quiz.blocks.len() + 1,
            )?;
            quiz.blocks
                .insert(position - 1, QuizBlock::Question { question });
        }
        2 if !groups.is_empty() => {
            let Some(block_index) = select_group(quiz, &groups)? else {
                quiz.blocks.push(QuizBlock::Question { question });
                println!("ยกเลิกการย้าย โจทย์ถูกเพิ่มกลับเป็นโจทย์เดี่ยวท้าย quiz");
                return Ok(());
            };
            let group_len = match &quiz.blocks[block_index] {
                QuizBlock::Group { questions, .. } => questions.len(),
                QuizBlock::Question { .. } => unreachable!("ต้องเลือกกลุ่ม"),
            };
            let position = read_number_in_range("แทรกเป็นโจทย์ลำดับที่: ", 1, group_len + 1)?;
            if let QuizBlock::Group { questions, .. } = &mut quiz.blocks[block_index] {
                questions.insert(position - 1, question);
            }
        }
        _ => unreachable!("ตัวเลือกปลายทางถูกตรวจช่วงแล้ว"),
    }
    Ok(())
}

/// ดึงโจทย์ออกจากตำแหน่งเดิม และลบบล็อกกลุ่มหากกลายเป็นกลุ่มว่าง
fn take_question(quiz: &mut Quiz, location: QuestionLocation) -> Question {
    match (&mut quiz.blocks[location.block_index], location.question_index) {
        (QuizBlock::Question { .. }, None) => match quiz.blocks.remove(location.block_index) {
            QuizBlock::Question { question } => question,
            QuizBlock::Group { .. } => unreachable!("ตำแหน่งโจทย์ต้องตรงกับชนิดบล็อก"),
        },
        (QuizBlock::Group { questions, .. }, Some(question_index)) => {
            let question = questions.remove(question_index);
            if questions.is_empty() {
                quiz.blocks.remove(location.block_index);
            }
            question
        }
        _ => unreachable!("ตำแหน่งโจทย์ต้องตรงกับชนิดบล็อก"),
    }
}

/// ย้ายโจทย์เดี่ยวหรือทั้งกลุ่มไปยังตำแหน่งใหม่ในระดับบล็อกหลัก
fn move_block(quiz: &mut Quiz) -> AppResult<()> {
    if quiz.blocks.is_empty() {
        println!("ไม่มีบล็อกให้ย้าย");
        return Ok(());
    }
    display_blocks(quiz);
    println!("0. ยกเลิก");
    let source = read_number_in_range("เลือกบล็อกที่ต้องการย้าย: ", 0, quiz.blocks.len())?;
    if source == 0 {
        println!("ยกเลิกการย้ายบล็อก");
        return Ok(());
    }
    let source = source - 1;
    let block = quiz.blocks.remove(source);
    display_blocks(quiz);
    println!("0. ยกเลิก (บล็อกจะถูกวางกลับที่เดิม)");
    let position = read_number_in_range(
        "แทรกก่อนบล็อกหมายเลขใด (หรือเลือกหลังสุด): ",
        0,
        quiz.blocks.len() + 1,
    )?;
    if position == 0 {
        quiz.blocks.insert(source, block);
        println!("ยกเลิกการย้าย บล็อกถูกวางกลับที่เดิม");
        return Ok(());
    }
    quiz.blocks.insert(position - 1, block);
    Ok(())
}

/// แสดงบล็อกระดับบนสุดเพื่อใช้กำหนดตำแหน่งย้าย
fn display_blocks(quiz: &Quiz) {
    println!("\nบล็อกตามลำดับ:");
    for (index, block) in quiz.blocks.iter().enumerate() {
        match block {
            QuizBlock::Question { question } => println!(
                "{}. โจทย์เดี่ยว: {}",
                index + 1,
                question.prompt.lines().next().unwrap_or("")
            ),
            QuizBlock::Group {
                title, questions, ..
            } => println!("{}. กลุ่ม: {title} ({} ข้อ)", index + 1, questions.len()),
        }
    }
    println!("ตำแหน่งหลังสุดคือ {}", quiz.blocks.len() + 1);
}

/// คืน index ของทุกกลุ่มตามลำดับที่อยู่ในบล็อกหลัก
fn group_block_indices(quiz: &Quiz) -> Vec<usize> {
    quiz.blocks
        .iter()
        .enumerate()
        .filter_map(|(index, block)| matches!(block, QuizBlock::Group { .. }).then_some(index))
        .collect()
}

/// แสดงรายชื่อกลุ่มและคืน index ของกลุ่มที่ผู้ใช้เลือก
fn select_group(quiz: &Quiz, groups: &[usize]) -> AppResult<Option<usize>> {
    for (index, &block_index) in groups.iter().enumerate() {
        if let QuizBlock::Group {title, questions, ..} = &quiz.blocks[block_index]
        {
            println!("{}. {} ({} ข้อ)", index + 1, title, questions.len());
        }
    }
    println!("0. ยกเลิก");
    let selected = read_number_in_range("เลือกกลุ่ม: ", 0, groups.len())?;
    if selected == 0 {
        return Ok(None);
    }
    Ok(Some(groups[selected - 1]))
}

/// แก้ชื่อและบทความของกลุ่มที่เลือกโดยไม่เปลี่ยนโจทย์ย่อย
fn edit_group(quiz: &mut Quiz) -> AppResult<()> {
    let groups = group_block_indices(quiz);
    if groups.is_empty() {
        println!("quiz นี้ยังไม่มีกลุ่ม");
        return Ok(());
    }
    let Some(block_index) = select_group(quiz, &groups)? else {
        println!("ยกเลิก");
        return Ok(());
    };
    let QuizBlock::Group {
        title,
        passage,
        ..
    } = &mut quiz.blocks[block_index]
    else {
        unreachable!("ต้องเลือกกลุ่ม")
    };
    println!("1) แก้ชื่อกลุ่ม");
    println!("2) แก้บทความ");
    println!("0) กลับ");
    match prompt("เลือก: ")?.trim() {
        "1" => *title = prompt_nonempty("ชื่อกลุ่มใหม่: ")?,
        "2" => {
            println!("กรอกบทความ จบด้วยบรรทัดที่มี . เพียงตัวเดียว");
            *passage = read_multiline()?;
        }
        _ => {}
    }
    Ok(())
}
