use rand::seq::SliceRandom;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::input::{prompt, read_number_in_range, read_yes_no};
use crate::model::{
    validate_quiz, AnswerSubmission, Question, QuestionLocation, Quiz, QuizBlock, SubmittedAnswer,
};
use crate::storage::{
    select_loaded_quiz, slug, submission_files, unique_path, LoadedQuiz, RESULT_DIR, SUBMISSION_DIR,
};
use crate::AppResult;

pub fn take_quiz() -> AppResult<()> {
    let Some(LoadedQuiz { quiz, .. }) = select_loaded_quiz("เลือก quiz: ")? else {
        return Ok(());
    };
    validate_quiz(&quiz)?;
    let student_name = prompt("ชื่อผู้ทำ (เว้นว่างได้): ")?.trim().to_string();
    let student_name = (!student_name.is_empty()).then_some(student_name);
    let shuffle_questions = read_yes_no("สลับลำดับโจทย์ (สลับเป็นกลุ่มทั้งก้อน) หรือไม่? (y/n): ")?;
    run_quiz(&quiz, student_name, shuffle_questions)
}

/// เลือกไฟล์ quiz เฉลยและไฟล์คำตอบ แล้วตรวจว่าทั้งสองชุดเป็นข้อสอบชุดเดียวกัน
pub fn grade_submission() -> AppResult<()> {
    println!("\nเลือก quiz ที่ใช้เป็นเฉลย:");
    let Some(LoadedQuiz { quiz: key_quiz, .. }) = select_loaded_quiz("เลือก quiz เฉลย: ")? else {
        return Ok(());
    };
    validate_quiz(&key_quiz)?;

    let submissions = submission_files()?;
    if submissions.is_empty() {
        println!("ยังไม่มีไฟล์คำตอบในโฟลเดอร์ {SUBMISSION_DIR}/");
        return Ok(());
    }

    println!("\nไฟล์คำตอบ:");
    for (index, path) in submissions.iter().enumerate() {
        println!("{}. {}", index + 1, path.display());
    }
    println!("0. ยกเลิก / กลับเมนูหลัก");
    let selected = read_number_in_range("เลือกไฟล์คำตอบ: ", 0, submissions.len())?;
    if selected == 0 {
        println!("ยกเลิก กลับสู่เมนูหลัก");
        return Ok(());
    }
    let submission: AnswerSubmission = serde_json::from_reader(File::open(&submissions[selected - 1])?)?;

    validate_submission_matches_quiz(&submission, &key_quiz)?;
    let report_path = write_imported_results(&key_quiz, &submission)?;
    print_grade_summary(&key_quiz, &submission, &report_path);
    Ok(())
}

fn run_quiz(quiz: &Quiz, student_name: Option<String>, shuffle_questions: bool) -> AppResult<()> {
    // แปลงบล็อกเป็นลำดับโจทย์ที่จะทำ โดยคงความสัมพันธ์ของโจทย์ในกลุ่มไว้
    let questions = prepare_questions(quiz, shuffle_questions);
    // แต่ละตำแหน่งใน choice_orders คือ index ของ choice เดิมที่นำมาแสดง
    let mut choice_orders: Vec<Vec<usize>> = questions
        .iter()
        .map(|view| (0..view.question(quiz).choices.len()).collect())
        .collect();
    // สุ่มเฉพาะข้อที่เจ้าของโจทย์อนุญาตให้สลับ choice
    for (index, view) in questions.iter().enumerate() {
        if view.question(quiz).shuffle_choices {
            choice_orders[index].shuffle(&mut rand::rng());
        }
    }
    // คำตอบที่ผู้ใช้เลือกเก็บเป็น index เดิม เพื่อให้ตรวจเฉลยได้แม้ choice ถูกสลับ
    let mut answers = vec![None; questions.len()];
    let mut current = 0;

    loop {
        display_question(quiz, current, &questions, &choice_orders, &answers);
        println!("คำสั่ง: เลข choice ตอบ | c ลบคำตอบ | j <ข้อ> ไปยังข้อ | < ก่อนหน้า | > ถัดไป | s ส่งตรวจ | q ยกเลิก | ? ช่วยเหลือ");
        let command = prompt("> ")?;
        let command = command.trim();
        // คำสั่งเลื่อนข้อจะไม่ทำให้ตำแหน่งปัจจุบันออกนอกช่วง
        match command {
            "<" if current > 0 => current -= 1,
            "<" => println!("อยู่ข้อแรกแล้ว"),
            ">" if current + 1 < questions.len() => current += 1,
            ">" => println!("อยู่ข้อสุดท้ายแล้ว"),
            "c" | "clear" | "ลบ" => {
                if answers[current].is_some() {
                    answers[current] = None;
                    println!("ลบคำตอบของข้อ {} แล้ว", current + 1);
                } else {
                    println!("ข้อนี้ยังไม่ได้เลือกคำตอบ");
                }
            }
            "s" => {
                let unanswered = unanswered_question_numbers(&answers);
                if !unanswered.is_empty() {
                    println!(
                        "ยังไม่ได้ตอบข้อ: {}",
                        unanswered
                            .iter()
                            .map(usize::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                if read_yes_no("ยืนยันส่งตรวจหรือไม่? (y/n): ")?
                {
                    break;
                }
            }
            _ if command.starts_with("j") => {
                // คำสั่ง j รับเลขข้อแบบ 1-based แล้วแปลงเป็น index แบบ 0-based
                match command[2..].trim().parse::<usize>() {
                    Ok(target) if target > 0 && target <= questions.len() => {
                        current = target - 1;
                    }
                    Ok(_) => println!("หมายเลขข้อต้องอยู่ระหว่าง 1 ถึง {}", questions.len()),
                    Err(_) => println!("รูปแบบคำสั่งไม่ถูกต้อง ใช้ j <ข้อ> เช่น j 3"),
                }
            }
            "?" | "help" => println!(
                "วิธีใช้: พิมพ์เลข choice เพื่อตอบ, c เพื่อลบคำตอบข้อนี้, j <ข้อ> เพื่อไปข้อที่ระบุ, \
                 < หรือ > เพื่อเลื่อนข้อ, s เพื่อส่งตรวจ, q เพื่อยกเลิกและกลับเมนูหลัก"
            ),
            "q" | "quit" | "cancel" | "ยกเลิก" => {
                if read_yes_no("ยืนยันยกเลิกการทำข้อสอบหรือไม่? (คำตอบจะไม่ถูกบันทึก) (y/n): ")? {
                    println!("ยกเลิกการทำข้อสอบแล้ว กลับสู่เมนูหลัก");
                    return Ok(());
                }
            }
            _ => match command.parse::<usize>() {
                // แปลงเลข choice ที่เห็นบนหน้าจอกลับเป็น index ของ choice เดิม
                Ok(choice) if choice > 0 && choice <= choice_orders[current].len() => {
                    answers[current] = Some(choice_orders[current][choice - 1]);
                }
                _ => println!("คำสั่งไม่ถูกต้อง หรือหมายเลข choice อยู่นอกช่วง"),
            },
        }
    }

    // การส่งตรวจจะสร้างรายงานโดยไม่แสดงเฉลยระหว่างที่ผู้ใช้กำลังทำ
    let submission = create_submission(quiz, student_name.clone(), &questions, &answers);
    let submission_path = write_submission(&submission)?;
    let result_path = write_results(quiz, &questions, &answers)?;
    print_quiz_summary(
        quiz,
        student_name.as_deref(),
        &questions,
        &answers,
        &submission_path,
        &result_path,
    );
    Ok(())
}

/// คลี่บล็อกเป็นโจทย์สำหรับทำ โดยสับเปลี่ยนเฉพาะลำดับบล็อกระดับบนสุด
fn prepare_questions(quiz: &Quiz, shuffle_questions: bool) -> Vec<QuestionView> {
    let mut blocks: Vec<usize> = (0..quiz.blocks.len()).collect();
    if shuffle_questions {
        blocks.shuffle(&mut rand::rng());
    }

    // ยุบโจทย์เดี่ยวและโจทย์ในกลุ่มเป็นรายการเดียวสำหรับระบบนำทาง
    let mut questions = Vec::new();
    for block_index in blocks {
        match &quiz.blocks[block_index] {
            QuizBlock::Question { .. } => questions.push(QuestionView {
                location: QuestionLocation {
                    block_index,
                    question_index: None,
                },
            }),
            QuizBlock::Group {
                questions: group_questions,
                ..
            } => {
                for question_index in 0..group_questions.len() {
                    questions.push(QuestionView {
                        location: QuestionLocation {
                            block_index,
                            question_index: Some(question_index),
                        },
                    });
                }
            }
        }
    }
    questions
}

/// คืนหมายเลขข้อแบบเริ่มจาก 1 ของข้อที่ยังไม่มีคำตอบ
fn unanswered_question_numbers(answers: &[Option<usize>]) -> Vec<usize> {
    answers
        .iter()
        .enumerate()
        .filter_map(|(index, answer)| answer.is_none().then_some(index + 1))
        .collect()
}

/// มุมมองชั่วคราวของโจทย์ที่กำลังทำ โดยเก็บตำแหน่งใน Quiz แทนการยืมข้อมูล
#[derive(Clone, Copy)]
struct QuestionView {
    location: QuestionLocation,
}

impl QuestionView {
    fn question(self, quiz: &Quiz) -> &Question {
        quiz.question_at(self.location)
            .expect("ตำแหน่งโจทย์ต้องมาจาก prepare_questions")
    }

    fn context(self, quiz: &Quiz) -> Option<(&str, &str)> {
        match quiz.blocks.get(self.location.block_index)? {
            QuizBlock::Question { .. } => None,
            QuizBlock::Group {
                title,
                passage,
                questions,
            } if self
                .location
                .question_index
                .is_some_and(|index| index < questions.len()) =>
            {
                Some((title.as_str(), passage.as_str()))
            }
            QuizBlock::Group { .. } => None,
        }
    }
}

/// สร้างข้อมูลคำตอบที่ไม่มีเฉลยหรือคำอธิบายปะปนอยู่ในไฟล์
fn create_submission(
    quiz: &Quiz,
    student_name: Option<String>,
    questions: &[QuestionView],
    answers: &[Option<usize>],
) -> AnswerSubmission {
    let answers = questions
        .iter()
        .zip(answers)
        .map(|(view, selected)| {
            let question = view.question(quiz);
            let context = view.context(quiz);
            SubmittedAnswer {
                block_index: view.location.block_index,
                question_index: view.location.question_index,
                group_title: context.map(|(title, _)| title.to_string()),
                passage: context.map(|(_, passage)| passage.to_string()),
                prompt: question.prompt.clone(),
                choices: question.choices.clone(),
                selected: *selected,
            }
        })
        .collect();

    AnswerSubmission {
        format_version: 1,
        quiz_title: quiz.title.clone(),
        student_name,
        answers,
    }
}

/// เขียนไฟล์ JSON สำหรับส่งให้ครูตรวจภายหลัง โดยไม่บันทึกข้อมูลเฉลย
fn write_submission(submission: &AnswerSubmission) -> AppResult<PathBuf> {
    fs::create_dir_all(SUBMISSION_DIR)?;
    let student = submission.student_name.as_deref().unwrap_or("student");
    let base = format!("{}-{}", slug(&submission.quiz_title), slug(student));
    let path = unique_path(Path::new(SUBMISSION_DIR), &base, "json");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    serde_json::to_writer_pretty(&mut file, submission)?;
    writeln!(file)?;
    Ok(path)
}

/// ตรวจ version, ตำแหน่ง และ snapshot ของโจทย์ก่อนนำคำตอบไปเทียบกับเฉลย
fn validate_submission_matches_quiz(submission: &AnswerSubmission, quiz: &Quiz) -> AppResult<()> {
    if submission.format_version != 1 {
        return Err(format!("ไม่รองรับไฟล์คำตอบ version {}", submission.format_version).into());
    }
    if submission.quiz_title != quiz.title {
        return Err(format!(
            "ไฟล์คำตอบเป็นของ quiz \"{}\" แต่เลือกเฉลย \"{}\"",
            submission.quiz_title, quiz.title
        )
        .into());
    }

    let expected = prepare_questions(quiz, false);
    if submission.answers.len() != expected.len() {
        return Err(format!(
            "จำนวนข้อไม่ตรงกัน: ไฟล์คำตอบมี {} ข้อ แต่เฉลยมี {} ข้อ",
            submission.answers.len(),
            expected.len()
        )
        .into());
    }

    let mut matched = vec![false; expected.len()];
    for submitted in &submission.answers {
        let Some((expected_index, view)) = expected.iter().enumerate().find(|(_, view)| 
        {
            view.location == (QuestionLocation {
                                block_index: submitted.block_index,
                                question_index: submitted.question_index,
                            })
        }) else {
            return Err("ตำแหน่งโจทย์ในไฟล์คำตอบไม่ตรงกับ quiz เฉลย".into());
        };

        if matched[expected_index] {
            return Err("ไฟล์คำตอบมีข้อมูลโจทย์ซ้ำ".into());
        }

        let question = view.question(quiz);
        let (group_title, passage) = view.context(quiz).map_or((None, None), |(title, passage)| {
            (Some(title), Some(passage))
        });
        if submitted.prompt != question.prompt
            || submitted.choices != question.choices
            || submitted.group_title.as_deref() != group_title
            || submitted.passage.as_deref() != passage
        {
            return Err(format!(
                "ข้อมูลโจทย์ข้อ \"{}\" ไม่ตรงกับ quiz เฉลย อาจเลือก quiz ผิดชุดหรือมีการแก้ไขโจทย์",
                submitted.prompt.lines().next().unwrap_or("")
            )
            .into());
        }
        if submitted
            .selected
            .is_some_and(|selected| selected >= submitted.choices.len())
        {
            return Err("พบหมายเลข choice ในไฟล์คำตอบที่อยู่นอกช่วง".into());
        }
        matched[expected_index] = true;
    }
    
    if matched.iter().any(|was_matched| !was_matched) {
        return Err("ไฟล์คำตอบมีโจทย์ไม่ครบ".into());
    }
    Ok(())
}

/// แสดงโจทย์และ choice ตามลำดับที่สุ่ม พร้อมทำเครื่องหมายคำตอบที่เลือกไว้
fn display_question(
    quiz: &Quiz,
    index: usize,
    questions: &[QuestionView],
    choice_orders: &[Vec<usize>],
    answers: &[Option<usize>],
) {
    let view = &questions[index];
    let answered_count = answers.iter().filter(|answer| answer.is_some()).count();
    println!(
        "\n--- ข้อ {}/{} | ตอบแล้ว {}/{} ---",
        index + 1,
        questions.len(),
        answered_count,
        questions.len()
    );
    let question = view.question(quiz);
    // แสดงบริบทซ้ำในทุกข้อของกลุ่ม เพื่อให้ผู้ใช้เห็นบทความประกอบเสมอ
    if let Some((title, passage)) = view.context(quiz) {
        println!("=== บทความของกลุ่ม: {title} ===");
        println!("{passage}");
        println!("========================================");
    }
    println!("--- โจทย์ปัจจุบัน ---");
    println!("{}", question.prompt);
    // original_index ใช้เชื่อม choice ที่แสดงกับตำแหน่งในข้อมูลต้นฉบับ
    for (display_index, &original_index) in choice_orders[index].iter().enumerate() {
        let marker = if answers[index] == Some(original_index) {
            " (เลือกแล้ว)"
        } else {
            ""
        };
        println!(
            "{}. {}{marker}",
            display_index + 1,
            question.choices[original_index]
        );
    }
}

/// ตรวจคำตอบทั้งหมดและเขียนคะแนน เฉลย และคำอธิบายลงไฟล์รายงาน
fn write_results(
    quiz: &Quiz,
    questions: &[QuestionView],
    answers: &[Option<usize>],
) -> AppResult<PathBuf> {
    fs::create_dir_all(RESULT_DIR)?;
    let path = unique_path(
        Path::new(RESULT_DIR),
        &format!("{}-results", slug(&quiz.title)),
        "txt",
    );
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    // นับจำนวนข้อที่มีเฉลย
    let graded_count = questions
        .iter()
        .filter(|view| view.question(quiz).answer.is_some())
        .count();
    // จำนวนข้อที่ตอบถูก
    let correct_count = questions
        .iter()
        .zip(answers)
        .filter(|(view, answer)| {
            view.question(quiz)
                .answer
                .is_some_and(|correct| **answer == Some(correct))
        })
        .count();

    writeln!(file, "ผลการทำ quiz: {}", quiz.title)?;
    writeln!(
        file,
        "คะแนน: {correct_count}/{graded_count} ข้อที่มีเฉลย (ทั้งหมด {} ข้อ)\n",
        questions.len()
    )?;
    for (index, (view, answer)) in questions.iter().zip(answers).enumerate() {
        let question = view.question(quiz);
        let result = match question.answer {
            Some(correct) if *answer == Some(correct) => "ถูก",
            Some(_) => "ผิด",
            None => "ยังไม่มีเฉลย",
        };
        writeln!(file, "ข้อ {}: {}", index + 1, result)?;
        if let Some((title, _)) = view.context(quiz) {
            writeln!(file, "กลุ่ม: {title}")?;
        }
        writeln!(file, "โจทย์: {}", question.prompt)?;
        // ระบุแยกกรณีไม่ได้ตอบ เพื่อไม่ให้รายงานตีความเป็น choice ว่าง
        match answer {
            Some(selected) => writeln!(file, "คำตอบของคุณ: {}", question.choices[*selected])?,
            None => writeln!(file, "คำตอบของคุณ: (ไม่ได้ตอบ)")?,
        }
        if let Some(correct) = question.answer {
            writeln!(file, "เฉลย: {}", question.choices[correct])?;
        } else {
            writeln!(file, "เฉลย: (ยังไม่ได้กำหนด)")?;
        }
        if let Some(explanation) = &question.explanation {
            writeln!(file, "คำอธิบาย: {explanation}")?;
        }
        writeln!(file)?;
    }
    Ok(path)
}

/// สร้างรายงานตรวจจาก submission ที่นำเข้ามาเทียบกับ quiz เฉลยที่เลือก
fn write_imported_results(quiz: &Quiz, submission: &AnswerSubmission) -> AppResult<PathBuf> {
    fs::create_dir_all(RESULT_DIR)?;
    let student = submission.student_name.as_deref().unwrap_or("student");
    let base = format!("{}-{}-graded", slug(&quiz.title), slug(student));
    let path = unique_path(Path::new(RESULT_DIR), &base, "txt");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    let expected = prepare_questions(quiz, false);
    let graded_count = submission
        .answers
        .iter()
        .filter(|answer| {
            expected.iter().any(|view| {
                view.location
                    == (QuestionLocation {
                        block_index: answer.block_index,
                        question_index: answer.question_index,
                    })
                    && view.question(quiz).answer.is_some()
            })
        })
        .count();
    let correct_count = submission
        .answers
        .iter()
        .filter(|answer| {
            expected.iter().any(|view| {
                view.location
                    == (QuestionLocation {
                        block_index: answer.block_index,
                        question_index: answer.question_index,
                    })
                    && view.question(quiz).answer == answer.selected
                    && view.question(quiz).answer.is_some()
            })
        })
        .count();

    writeln!(file, "ผลตรวจ quiz: {}", quiz.title)?;
    writeln!(
        file,
        "ผู้ทำ: {}",
        submission.student_name.as_deref().unwrap_or("(ไม่ระบุ)")
    )?;
    writeln!(
        file,
        "คะแนน: {correct_count}/{graded_count} ข้อที่มีเฉลย (ทั้งหมด {} ข้อ)\n",
        submission.answers.len()
    )?;

    for (index, submitted) in submission.answers.iter().enumerate() {
        let view = expected
            .iter()
            .find(|view| {
                view.location
                    == (QuestionLocation {
                        block_index: submitted.block_index,
                        question_index: submitted.question_index,
                    })
            })
            .ok_or("ไม่พบโจทย์ที่ตรงกับไฟล์คำตอบหลังตรวจสอบ")?;
        let question = view.question(quiz);
        let status = match question.answer {
            Some(correct) if submitted.selected == Some(correct) => "ถูก",
            Some(_) => "ผิด",
            None => "ยังไม่มีเฉลย",
        };
        writeln!(file, "ข้อ {}: {status}", index + 1)?;
        if let Some((title, _)) = view.context(quiz) {
            writeln!(file, "กลุ่ม: {title}")?;
        }
        writeln!(file, "โจทย์: {}", question.prompt)?;
        match submitted.selected {
            Some(selected) => {
                let choice_text = question
                    .choices
                    .get(selected)
                    .map_or("(ตัวเลือกไม่ถูกต้อง)", |s| s.as_str());
                writeln!(file, "คำตอบ: {choice_text}")?;
            }
            None => writeln!(file, "คำตอบ: (ไม่ได้ตอบ)")?,
        }
        if let Some(correct) = question.answer {
            writeln!(file, "เฉลย: {}", question.choices[correct])?;
        } else {
            writeln!(file, "เฉลย: (ยังไม่ได้กำหนด)")?;
        }
        if let Some(explanation) = &question.explanation {
            writeln!(file, "คำอธิบาย: {explanation}")?;
        }
        writeln!(file)?;
    }
    Ok(path)
}

/// แสดงสรุปผลคะแนนบน terminal เมื่อทำ quiz เสร็จ
fn print_quiz_summary(
    quiz: &Quiz,
    student_name: Option<&str>,
    questions: &[QuestionView],
    answers: &[Option<usize>],
    submission_path: &Path,
    result_path: &Path,
) {
    let graded_count = questions
        .iter()
        .filter(|view| view.question(quiz).answer.is_some())
        .count();
    let correct_count = questions
        .iter()
        .zip(answers)
        .filter(|(view, answer)| {
            view.question(quiz)
                .answer
                .is_some_and(|correct| **answer == Some(correct))
        })
        .count();
    let unanswered_count = answers.iter().filter(|a| a.is_none()).count();
    let incorrect_count = answers
        .iter()
        .zip(questions)
        .filter(|(answer, view)| {
            view.question(quiz)
                .answer
                .is_some_and(|correct| answer.is_some() && **answer != Some(correct))
        })
        .count();

    println!("\n========================================");
    println!("           สรุปผลการทำ Quiz");
    println!("========================================");
    println!(" Quiz: {}", quiz.title);
    if let Some(name) = student_name {
        println!(" ผู้ทำ: {name}");
    }
    println!("----------------------------------------");
    println!(" จำนวนข้อทั้งหมด:  {} ข้อ", questions.len());
    println!(" ข้อที่มีเฉลย:     {graded_count} ข้อ");
    println!(" ตอบถูก:          {correct_count} ข้อ");
    println!(" ตอบผิด:          {incorrect_count} ข้อ");
    println!(" ไม่ได้ตอบ:        {unanswered_count} ข้อ");
    if graded_count > 0 {
        let percentage = (correct_count as f64 / graded_count as f64) * 100.0;
        println!(" คะแนนที่ได้:      {correct_count}/{graded_count} ({percentage:.1}%)");
    } else {
        println!(" คะแนนที่ได้:      (ไม่มีข้อที่มีเฉลย)");
    }
    println!("========================================");
    println!(" บันทึกไฟล์คำตอบ JSON: {}", submission_path.display());
    println!(" บันทึกรายงานผลอย่างละเอียด: {}", result_path.display());
    println!("========================================\n");
}

/// แสดงสรุปผลคะแนนบน terminal เมื่อตรวจไฟล์คำตอบ
fn print_grade_summary(
    quiz: &Quiz,
    submission: &AnswerSubmission,
    report_path: &Path,
) {
    let expected = prepare_questions(quiz, false);
    let graded_count = submission
        .answers
        .iter()
        .filter(|answer| {
            expected.iter().any(|view| {
                view.location
                    == (QuestionLocation {
                        block_index: answer.block_index,
                        question_index: answer.question_index,
                    })
                    && view.question(quiz).answer.is_some()
            })
        })
        .count();
    let correct_count = submission
        .answers
        .iter()
        .filter(|answer| {
            expected.iter().any(|view| {
                view.location
                    == (QuestionLocation {
                        block_index: answer.block_index,
                        question_index: answer.question_index,
                    })
                    && view.question(quiz).answer == answer.selected
                    && view.question(quiz).answer.is_some()
            })
        })
        .count();
    let answered_count = submission.answers.iter().filter(|a| a.selected.is_some()).count();
    let unanswered_count = submission.answers.len().saturating_sub(answered_count);
    let incorrect_count = graded_count.saturating_sub(correct_count);

    println!("\n========================================");
    println!("           สรุปผลการตรวจ Quiz");
    println!("========================================");
    println!(" Quiz: {}", quiz.title);
    println!(" ผู้ทำ: {}", submission.student_name.as_deref().unwrap_or("(ไม่ระบุ)"));
    println!("----------------------------------------");
    println!(" จำนวนข้อทั้งหมด:  {} ข้อ", submission.answers.len());
    println!(" ข้อที่มีเฉลย:     {graded_count} ข้อ");
    println!(" ตอบถูก:          {correct_count} ข้อ");
    println!(" ตอบผิด:          {incorrect_count} ข้อ");
    println!(" ไม่ได้ตอบ:        {unanswered_count} ข้อ");
    if graded_count > 0 {
        let percentage = (correct_count as f64 / graded_count as f64) * 100.0;
        println!(" คะแนนที่ได้:      {correct_count}/{graded_count} ({percentage:.1}%)");
    } else {
        println!(" คะแนนที่ได้:      (ไม่มีข้อที่มีเฉลย)");
    }
    println!("========================================");
    println!(" บันทึกรายงานผลตรวจ: {}", report_path.display());
    println!("========================================\n");
}
