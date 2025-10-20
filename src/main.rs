use rayon::prelude::*;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::time::Instant;

// --- Constants ---
const FULL_LEN: usize = 10;
const NUM_ELEMENTS: usize = 5;

// --- Element Definition and Mappings ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Element {
    Thuy = 0,
    Tho = 1,
    Moc = 2,
    Kim = 3,
    Hoa = 4,
}

fn digit_to_element(d: u8) -> Element {
    match d {
        1 => Element::Thuy,
        0 | 2 | 5 | 8 => Element::Tho,
        3 | 4 => Element::Moc,
        6 | 7 => Element::Kim,
        9 => Element::Hoa,
        _ => unreachable!(),
    }
}

const RELATION_MATRIX: [[i8; NUM_ELEMENTS]; NUM_ELEMENTS] = [
    /*Thủy*/ [0, -1, 1, 0, -1],
    /*Thổ */ [1, 0, -1, 0, 1],
    /*Mộc */ [0, 1, 0, -1, 1],
    /*Kim */ [1, 0, 1, 0, -1],
    /*Hỏa */ [-1, 1, 0, 1, 0],
];

// --- PhoneNumber Struct and Logic ---

#[derive(Clone, Copy)]
struct PhoneNumber {
    original_digits: [u8; FULL_LEN],
}

impl PhoneNumber {
    fn from_digits(digits_slice: &[u8]) -> Option<Self> {
        if digits_slice.len() != FULL_LEN {
            return None;
        }
        let mut digits = [0; FULL_LEN];
        digits.copy_from_slice(digits_slice);
        Some(Self { original_digits: digits })
    }

    // --- Stage 0: Pre-processing ---
    fn get_transformed_digits(&self) -> [u8; FULL_LEN] {
        let mut transformed = self.original_digits;
        for digit in &mut transformed {
            if *digit == 0 {
                *digit = 5;
            }
        }
        transformed
    }

    // --- Stage 1: Filters ---

    fn has_static_balance(&self) -> bool {
        // Parity check on original digits
        let even_count = self.original_digits.iter().filter(|&&d| d % 2 == 0).count();
        if !(even_count == 4 || even_count == 5) {
            return false;
        }

        // Sum check on original digits
        let sum_first5: u32 = self.original_digits[..5].iter().map(|&d| d as u32).sum();
        let sum_last5: u32 = self.original_digits[5..].iter().map(|&d| d as u32).sum();
        if sum_first5 % 8 == 0 || sum_last5 % 8 == 0 {
            return false;
        }

        true
    }

    fn passes_elemental_filters(&self, transformed_digits: &[u8; FULL_LEN], user_menh: Element) -> bool {
        let mut counts = [0; NUM_ELEMENTS];
        for &digit in transformed_digits {
            counts[digit_to_element(digit) as usize] += 1;
        }

        // 4. Completeness Filter
        if counts.iter().any(|&c| c == 0) {
            return false;
        }

        // 5. Dominance Filter
        if counts.iter().any(|&c| c > 4) {
            return false;
        }

        // 6. Dynamic Balance Filter
        let (sinh, cung, bi_khac, khac, _sinh_xuat) = get_element_roles(user_menh);
        let count_sinh = counts[sinh as usize];
        let count_cung = counts[cung as usize];
        let count_khac = counts[khac as usize];
        let count_bi_khac = counts[bi_khac as usize];

        if count_khac != 1 { return false; }         // 6a
        if count_bi_khac > 2 { return false; }        // NEW RULE
        if count_sinh < 2 { return false; }          // 6b
        if count_cung < 2 { return false; }          // 6c
        if (count_sinh + count_cung) > 5 { return false; } // 6d

        true
    }

    // --- Stage 2: Scoring ---

    fn calculate_adjacent_score(&self, transformed_digits: &[u8; FULL_LEN]) -> f64 {
        transformed_digits
            .windows(2)
            .map(|w| {
                let a = digit_to_element(w[0]) as usize;
                let b = digit_to_element(w[1]) as usize;
                RELATION_MATRIX[a][b] as i32
            })
            .sum::<i32>() as f64
    }

    fn calculate_compatibility_score(&self, transformed_digits: &[u8; FULL_LEN], user_menh: Element) -> f64 {
        let (sinh, cung, bi_khac, khac, sinh_xuat) = get_element_roles(user_menh);
        let mut score = 0.0;
        for &digit in transformed_digits {
            let element = digit_to_element(digit);
            let weight = if element == sinh {
                3.0
            }
            else if element == cung {
                2.0
            }
            else if element == khac {
                -3.0
            }
            else if element == sinh_xuat {
                -1.0
            }
            else { // bi_khac
                1.0
            };
            score += weight;
        }
        score
    }

    // --- Utility ---
    fn to_string_with_stats(&self, final_score: f64) -> String {
        let s: String = self.original_digits.iter().map(|d| (b'0' + *d) as char).collect();
        format!("{}  score={:.2}\n", s, final_score)
    }
}

// --- Main Application Logic ---

fn get_element_roles(menh: Element) -> (Element, Element, Element, Element, Element) {
    match menh {
        Element::Kim => (Element::Tho, Element::Kim, Element::Moc, Element::Hoa, Element::Thuy),
        Element::Moc => (Element::Thuy, Element::Moc, Element::Tho, Element::Kim, Element::Hoa),
        Element::Thuy => (Element::Kim, Element::Thuy, Element::Hoa, Element::Tho, Element::Moc),
        Element::Hoa => (Element::Moc, Element::Hoa, Element::Kim, Element::Thuy, Element::Tho),
        Element::Tho => (Element::Hoa, Element::Tho, Element::Thuy, Element::Moc, Element::Kim),
    }
}

fn get_user_menh() -> Element {
    loop {
        println!("Vui lòng nhập Bản Mệnh của bạn:");
        println!("  1: Kim");
        println!("  2: Mộc");
        println!("  3: Thủy");
        println!("  4: Hỏa");
        println!("  5: Thổ");
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read line");
        match input.trim().parse::<usize>() {
            Ok(1) => return Element::Kim,
            Ok(2) => return Element::Moc,
            Ok(3) => return Element::Thuy,
            Ok(4) => return Element::Hoa,
            Ok(5) => return Element::Tho,
            _ => println!("Lựa chọn không hợp lệ, vui lòng nhập một số từ 1 đến 5."),
        }
    }
}

fn main() -> io::Result<()> {
    let start_time = Instant::now();

    // --- Stage 0 & 1: Input & Setup ---
    let user_menh = get_user_menh();
    println!("Đã chọn mệnh: {:?}\n", user_menh);

    let input_filename = "sodienthoai.txt";
    let content = match fs::read_to_string(input_filename) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Lỗi: Không thể tìm thấy hoặc đọc file '{}'.", input_filename);
            return Ok(());
        }
    };
    let lines: Vec<&str> = content.lines().collect();

    // --- Stage 1 (cont.) & 2 & 3: Filtering and Scoring ---
    let mut results: Vec<(PhoneNumber, f64)> = lines
        .par_iter()
        .filter_map(|line| {
            let digits_str: String = line.chars().filter(|c| c.is_ascii_digit()).collect();
            if digits_str.len() != FULL_LEN { return None; }

            let phone = PhoneNumber::from_digits(
                &digits_str.chars().map(|c| c.to_digit(10).unwrap() as u8).collect::<Vec<u8>>(),
            )?;

            // Stage 1A: Static Balance Filter
            if !phone.has_static_balance() { return None; }

            // Stage 0: Transformation
            let transformed_digits = phone.get_transformed_digits();

            // Stage 1B: Elemental Filters
            if !phone.passes_elemental_filters(&transformed_digits, user_menh) { return None; }

            // Stage 2: Scoring
            let score_adj = phone.calculate_adjacent_score(&transformed_digits);
            let score_comp = phone.calculate_compatibility_score(&transformed_digits, user_menh);
            let final_score = (score_adj * 0.4) + (score_comp * 0.6);
            
            Some((phone, final_score))
        })
        .collect();

    println!(
        "Đã tìm thấy {} số hợp lệ sau khi lọc. Đang sắp xếp và ghi ra file...",
        results.len()
    );

    // Sort results by final score, descending
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // --- Stage 3: Output ---
    let mut writer = BufWriter::new(File::create("result.txt")?);
    for (phone, final_score) in results {
        let line = phone.to_string_with_stats(final_score);
        writer.write_all(line.as_bytes())?;
    }

    let elapsed = start_time.elapsed();
    println!("Hoàn thành!");
    println!("Đã ghi kết quả đã sắp xếp vào result.txt");
    println!("Thời gian chạy: {:.2?}", elapsed);

    Ok(())
}

