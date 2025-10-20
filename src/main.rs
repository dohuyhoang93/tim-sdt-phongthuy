use rayon::prelude::*;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::time::Instant;

// --- Constants ---
const FULL_LEN: usize = 10;
const NUM_ELEMENTS: usize = 5;

// --- Enum Definitions ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Element {
    Thuy = 0,
    Tho = 1,
    Moc = 2,
    Kim = 3,
    Hoa = 4,
}

impl Element {
}

#[derive(Debug, Clone, Copy)]
enum AnalysisMode {
    Compatibility,
    AbsoluteBalance,
}

// --- Element Mappings & Matrix ---

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
    [0, -1, 1, 0, -1], [1, 0, -1, 0, 1], [0, 1, 0, -1, 1], [1, 0, 1, 0, -1], [-1, 1, 0, 1, 0],
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

    fn get_transformed_digits(&self) -> [u8; FULL_LEN] {
        let mut transformed = self.original_digits;
        for digit in &mut transformed {
            if *digit == 0 {
                *digit = 5;
            }
        }
        transformed
    }

    // --- Filters ---

    fn has_static_balance(&self) -> bool {
        let even_count = self.original_digits.iter().filter(|&&d| d % 2 == 0).count();
        if !(even_count == 4 || even_count == 5) { return false; }

        let sum_first5: u32 = self.original_digits[..5].iter().map(|&d| d as u32).sum();
        let sum_last5: u32 = self.original_digits[5..].iter().map(|&d| d as u32).sum();
        if sum_first5 % 8 == 0 || sum_last5 % 8 == 0 { return false; }

        true
    }

    fn passes_compatibility_filters(&self, t_digits: &[u8; FULL_LEN], user_menh: Element) -> bool {
        let mut counts = [0; NUM_ELEMENTS];
        for &digit in t_digits {
            counts[digit_to_element(digit) as usize] += 1;
        }

        if counts.iter().any(|&c| c == 0) { return false; } // Completeness
        if counts.iter().any(|&c| c > 4) { return false; } // Dominance

        let (sinh, cung, bi_khac, khac, _) = get_element_roles(user_menh);
        if counts[khac as usize] != 1 { return false; }
        if counts[bi_khac as usize] > 2 { return false; }
        if counts[sinh as usize] < 2 { return false; }
        if counts[cung as usize] < 2 { return false; }
        if (counts[sinh as usize] + counts[cung as usize]) > 5 { return false; }

        true
    }

    fn has_absolute_balance(&self, t_digits: &[u8; FULL_LEN]) -> bool {
        let mut counts = [0; NUM_ELEMENTS];
        for &digit in t_digits {
            counts[digit_to_element(digit) as usize] += 1;
        }
        counts.iter().all(|&c| c == 2)
    }

    // --- Scoring ---

    fn calculate_adjacent_score(&self, t_digits: &[u8; FULL_LEN]) -> f64 {
        t_digits.windows(2).map(|w| {
            let a = digit_to_element(w[0]) as usize;
            let b = digit_to_element(w[1]) as usize;
            RELATION_MATRIX[a][b] as i32
        }).sum::<i32>() as f64
    }

    fn calculate_compatibility_score(&self, t_digits: &[u8; FULL_LEN], user_menh: Element) -> f64 {
        let (sinh, cung, bi_khac, khac, sinh_xuat) = get_element_roles(user_menh);
        t_digits.iter().map(|&digit| {
            let element = digit_to_element(digit);
            if element == sinh { 3.0 }
            else if element == cung { 2.0 }
            else if element == khac { -3.0 }
            else if element == sinh_xuat { -1.0 }
            else { 1.0 } // bi_khac
        }).sum()
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

fn get_user_input<T: std::str::FromStr>(prompt: &str) -> T {
    loop {
        println!("{}", prompt);
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read line");
        match input.trim().parse::<T>() {
            Ok(val) => return val,
            Err(_) => println!("Lựa chọn không hợp lệ, vui lòng thử lại."),
        }
    }
}

fn main() -> io::Result<()> {
    let start_time = Instant::now();

    let mode = loop {
        let choice = get_user_input("Vui lòng chọn chế độ:\n  1: Phân tích Tương hợp Bản mệnh\n  2: Tìm số Cân bằng Tuyệt đối");
        match choice {
            1 => break AnalysisMode::Compatibility,
            2 => break AnalysisMode::AbsoluteBalance,
            _ => println!("Chỉ nhập 1 hoặc 2."),
        }
    };

    let content = match fs::read_to_string("sodienthoai.txt") {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Lỗi: Không thể tìm thấy hoặc đọc file 'sodienthoai.txt'.");
            return Ok(());
        }
    };
    let lines: Vec<&str> = content.lines().collect();

    let mut results: Vec<(PhoneNumber, f64)> = match mode {
        AnalysisMode::Compatibility => {
            let user_menh = loop {
                let choice: usize = get_user_input("Vui lòng nhập Bản Mệnh:\n  1: Kim  2: Mộc  3: Thủy  4: Hỏa  5: Thổ");
                match choice {
                    1 => break Element::Kim,
                    2 => break Element::Moc,
                    3 => break Element::Thuy,
                    4 => break Element::Hoa,
                    5 => break Element::Tho,
                    _ => println!("Chỉ nhập 1 đến 5."),
                }
            };
            println!("Đã chọn chế độ Tương hợp Bản mệnh: {:?}\n", user_menh);

            lines.par_iter().filter_map(|line| {
                let phone = PhoneNumber::from_digits(&line.chars().filter(|c| c.is_ascii_digit()).map(|c| c.to_digit(10).unwrap() as u8).collect::<Vec<u8>>())?;
                if !phone.has_static_balance() { return None; }
                let t_digits = phone.get_transformed_digits();
                if !phone.passes_compatibility_filters(&t_digits, user_menh) { return None; }

                let score_adj = phone.calculate_adjacent_score(&t_digits);
                let score_comp = phone.calculate_compatibility_score(&t_digits, user_menh);
                Some((phone, (score_adj * 0.4) + (score_comp * 0.6)))
            }).collect()
        }
        AnalysisMode::AbsoluteBalance => {
            println!("Đã chọn chế độ Cân bằng Tuyệt đối.\n");
            lines.par_iter().filter_map(|line| {
                let phone = PhoneNumber::from_digits(&line.chars().filter(|c| c.is_ascii_digit()).map(|c| c.to_digit(10).unwrap() as u8).collect::<Vec<u8>>())?;
                if !phone.has_static_balance() { return None; }
                let t_digits = phone.get_transformed_digits();
                if !phone.has_absolute_balance(&t_digits) { return None; }

                let score_adj = phone.calculate_adjacent_score(&t_digits);
                Some((phone, score_adj))
            }).collect()
        }
    };

    println!("Đã tìm thấy {} số hợp lệ. Đang sắp xếp và ghi ra file...", results.len());
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut writer = BufWriter::new(File::create("result.txt")?);
    for (phone, final_score) in results {
        writer.write_all(phone.to_string_with_stats(final_score).as_bytes())?;
    }

    println!("Hoàn thành! Đã ghi kết quả vào result.txt");
    println!("Thời gian chạy: {:.2?}", start_time.elapsed());

    Ok(())
}


