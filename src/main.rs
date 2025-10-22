use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use std::collections::HashSet;

// --- Enums ---
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Element { Thuy = 0, Tho = 1, Moc = 2, Kim = 3, Hoa = 4, }

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AnalysisMode {
    Compatibility,
    AbsoluteBalance,
}

// --- JS Communication Structs ---

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct AnalyzeConfig {
    mode: AnalysisMode,
    user_menh: Element,
    // Score weights
    score_sinh: f64,
    score_cung: f64,
    score_bi_khac: f64,
    score_sinh_xuat: f64,
    score_khac: f64,
    // Filter thresholds
    filter_khac_max: usize,
    filter_bi_khac_max: usize,
    filter_sinh_min: usize,
    filter_cung_min: usize,
    filter_tong_max: usize,
    filter_any_max: usize,
    // Toggles
    toggle_static_balance: bool,
    toggle_completeness: bool,

    // Custom Filters
    toggle_prefix_filter: bool,
    prefix_value: String,
    toggle_suffix_filter: bool,
    suffix_value: String,
    toggle_blacklist_filter: bool,
    blacklist_digits: String,
}

impl Default for AnalyzeConfig {
    fn default() -> Self {
        AnalyzeConfig {
            mode: AnalysisMode::Compatibility,
            user_menh: Element::Kim,
            score_sinh: 3.0, score_cung: 2.0, score_bi_khac: 1.0, score_sinh_xuat: -1.0, score_khac: -3.0,
            filter_khac_max: 1, filter_bi_khac_max: 2, filter_sinh_min: 2, 
            filter_cung_min: 2, filter_tong_max: 5, filter_any_max: 4,
            toggle_static_balance: true, toggle_completeness: true,
            // Custom Filters
            toggle_prefix_filter: false,
            prefix_value: String::new(),
            toggle_suffix_filter: false,
            suffix_value: String::new(),
            toggle_blacklist_filter: false,
            blacklist_digits: String::new(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AnalyzeResult {
    number: String,
    score: f64,
}

#[derive(Debug, Serialize)]
pub enum CheckResult {
    Valid { score: f64 },
    Invalid { reason: String },
}

// --- Core Logic ---

fn digit_to_element(d: u8) -> Element {
    match d {
        1 => Element::Thuy, 0 | 2 | 5 | 8 => Element::Tho, 3 | 4 => Element::Moc,
        6 | 7 => Element::Kim, 9 => Element::Hoa, _ => unreachable!(),
    }
}

const RELATION_MATRIX: [[i8; 5]; 5] = [
    [0, -1, 1, 0, -1], [1, 0, -1, 0, 1], [0, 1, 0, -1, 1], [1, 0, 1, 0, -1], [-1, 1, 0, 1, 0],
];

#[derive(Clone, Copy)]
struct PhoneNumber { original_digits: [u8; 10] }

impl PhoneNumber {
    fn from_digits(slice: &[u8]) -> Option<Self> {
        if slice.len() != 10 { None } else { let mut arr = [0; 10]; arr.copy_from_slice(slice); Some(Self { original_digits: arr }) }
    }

    fn get_transformed_digits(&self) -> [u8; 10] {
        let mut t = self.original_digits; t.iter_mut().for_each(|d| if *d == 0 { *d = 5 }); t
    }

    fn has_static_balance(&self) -> bool {
        let even_count = self.original_digits.iter().filter(|&&d| d % 2 == 0).count();
        if !(even_count == 4 || even_count == 5) { return false; }
        let sum_first5: u32 = self.original_digits[..5].iter().map(|&d| d as u32).sum();
        let sum_last5: u32 = self.original_digits[5..].iter().map(|&d| d as u32).sum();
        if sum_first5 % 8 == 0 || sum_last5 % 8 == 0 { return false; }
        true
    }

    fn passes_compatibility_filters(&self, t_digits: &[u8; 10], cfg: &AnalyzeConfig) -> bool {
        let mut counts = [0; 5]; t_digits.iter().for_each(|&d| counts[digit_to_element(d) as usize] += 1);

        if cfg.toggle_completeness && counts.iter().any(|&c| c == 0) { return false; }
        if counts.iter().any(|&c| c > cfg.filter_any_max) { return false; }

        let (sinh, cung, bi_khac, khac, _) = get_element_roles(cfg.user_menh);
        if counts[khac as usize] > cfg.filter_khac_max { return false; }
        if counts[bi_khac as usize] > cfg.filter_bi_khac_max { return false; }
        if counts[sinh as usize] < cfg.filter_sinh_min { return false; }
        if counts[cung as usize] < cfg.filter_cung_min { return false; }
        if (counts[sinh as usize] + counts[cung as usize]) > cfg.filter_tong_max { return false; }

        true
    }

    fn has_absolute_balance(&self, t_digits: &[u8; 10]) -> bool {
        let mut counts = [0; 5]; t_digits.iter().for_each(|&d| counts[digit_to_element(d) as usize] += 1);
        counts.iter().all(|&c| c == 2)
    }

    fn calculate_adjacent_score(&self, t_digits: &[u8; 10]) -> f64 {
        t_digits.windows(2).map(|w| RELATION_MATRIX[digit_to_element(w[0]) as usize][digit_to_element(w[1]) as usize] as i32).sum::<i32>() as f64
    }

    fn calculate_compatibility_score(&self, t_digits: &[u8; 10], cfg: &AnalyzeConfig) -> f64 {
        let (sinh, cung, bi_khac, khac, sinh_xuat) = get_element_roles(cfg.user_menh);
        t_digits.iter().map(|&d| {
            let el = digit_to_element(d);
            if el == sinh { cfg.score_sinh }
            else if el == cung { cfg.score_cung }
            else if el == khac { cfg.score_khac }
            else if el == sinh_xuat { cfg.score_sinh_xuat }
            else if el == bi_khac { cfg.score_bi_khac }
            else { unreachable!() } // All 5 elements are covered, this should not be reached.
        }).sum()
    }
}

fn get_element_roles(menh: Element) -> (Element, Element, Element, Element, Element) {
    match menh {
        Element::Kim => (Element::Tho, Element::Kim, Element::Moc, Element::Hoa, Element::Thuy),
        Element::Moc => (Element::Thuy, Element::Moc, Element::Tho, Element::Kim, Element::Hoa),
        Element::Thuy => (Element::Kim, Element::Thuy, Element::Hoa, Element::Tho, Element::Moc),
        Element::Hoa => (Element::Moc, Element::Hoa, Element::Kim, Element::Thuy, Element::Tho),
        Element::Tho => (Element::Hoa, Element::Tho, Element::Thuy, Element::Moc, Element::Kim),
    }
}

#[wasm_bindgen]
pub fn analyze(phone_numbers_str: &str, config_js: JsValue) -> JsValue {
    let config: AnalyzeConfig = serde_wasm_bindgen::from_value(config_js).unwrap_or_default();
    let lines: Vec<&str> = phone_numbers_str.lines().collect();

    let mut results: Vec<AnalyzeResult> = lines.iter().filter_map(|line| {
        // --- Start of unified filtering logic ---

        // 1. Parse digits first. If it's not a 10-digit number, skip it.
        let phone_part = line.split_whitespace().next().unwrap_or("");
        let all_digits_str: String = phone_part.chars().filter(|c| c.is_ascii_digit()).collect();
        if all_digits_str.len() != 10 {
            return None;
        }
        let phone = PhoneNumber::from_digits(&all_digits_str.bytes().map(|b| b - b'0').collect::<Vec<u8>>())?;

        // 2. Apply Custom Filters
        // Prefix Filter
        if config.toggle_prefix_filter && !config.prefix_value.is_empty() {
            if !all_digits_str.starts_with(&config.prefix_value) {
                return None;
            }
        }
        // New Suffix Filter Logic
        if config.toggle_suffix_filter && !config.suffix_value.is_empty() {
            let last_3_digits: HashSet<u8> = phone.original_digits[7..10].iter().cloned().collect();
            let required_digits: HashSet<u8> = config.suffix_value
                .split(',')
                .filter_map(|s| s.trim().parse::<u8>().ok())
                .collect();

            if !required_digits.is_subset(&last_3_digits) {
                return None;
            }
        }
        // New Blacklist Filter Logic
        if config.toggle_blacklist_filter && !config.blacklist_digits.is_empty() {
            let forbidden_digits: HashSet<u8> = config.blacklist_digits
                .split(',')
                .filter_map(|s| s.trim().parse::<u8>().ok())
                .collect();
            if !forbidden_digits.is_empty() && phone.original_digits.iter().any(|d| forbidden_digits.contains(d)) {
                return None;
            }
        }

        // 3. Apply Static Balance Filter
        if config.toggle_static_balance && !phone.has_static_balance() {
            return None;
        }

        // --- End of unified filtering logic ---

        // Now, apply mode-specific logic
        match config.mode {
            AnalysisMode::Compatibility => {
                let t_digits = phone.get_transformed_digits();
                if !phone.passes_compatibility_filters(&t_digits, &config) { return None; }

                let score_adj = phone.calculate_adjacent_score(&t_digits);
                let score_comp = phone.calculate_compatibility_score(&t_digits, &config);
                let final_score = (score_adj * 0.4) + (score_comp * 0.6);
                let s: String = phone.original_digits.iter().map(|d| (b'0' + d) as char).collect();
                Some(AnalyzeResult { number: s, score: final_score })
            }
            AnalysisMode::AbsoluteBalance => {
                let t_digits = phone.get_transformed_digits();
                if !phone.has_absolute_balance(&t_digits) { return None; }

                let score_adj = phone.calculate_adjacent_score(&t_digits);
                let s: String = phone.original_digits.iter().map(|d| (b'0' + d) as char).collect();
                Some(AnalyzeResult { number: s, score: score_adj })
            }
        }
    }).collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    serde_wasm_bindgen::to_value(&results).unwrap()
}

#[wasm_bindgen]
pub fn quick_check_single_number(number_str: &str, config_js: JsValue) -> JsValue {
    let config: AnalyzeConfig = serde_wasm_bindgen::from_value(config_js).unwrap_or_default();

    // 1. Parse digits
    let phone_part = number_str.split_whitespace().next().unwrap_or("");
    let all_digits_str: String = phone_part.chars().filter(|c| c.is_ascii_digit()).collect();
    if all_digits_str.len() != 10 {
        return serde_wasm_bindgen::to_value(&CheckResult::Invalid { reason: "Số phải có 10 chữ số.".to_string() }).unwrap();
    }
    let phone = match PhoneNumber::from_digits(&all_digits_str.bytes().map(|b| b - b'0').collect::<Vec<u8>>()) {
        Some(p) => p,
        None => return serde_wasm_bindgen::to_value(&CheckResult::Invalid { reason: "Không thể phân tích cú pháp số.".to_string() }).unwrap(),
    };

    // 2. Apply Custom Filters
    if config.toggle_prefix_filter && !config.prefix_value.is_empty() {
        if !all_digits_str.starts_with(&config.prefix_value) {
            return serde_wasm_bindgen::to_value(&CheckResult::Invalid { reason: format!("Không khớp Prefix '{}'", config.prefix_value) }).unwrap();
        }
    }
    if config.toggle_suffix_filter && !config.suffix_value.is_empty() {
        let last_3_digits: HashSet<u8> = phone.original_digits[7..10].iter().cloned().collect();
        let required_digits: HashSet<u8> = config.suffix_value
            .split(',')
            .filter_map(|s| s.trim().parse::<u8>().ok())
            .collect();
        if !required_digits.is_subset(&last_3_digits) {
            return serde_wasm_bindgen::to_value(&CheckResult::Invalid { reason: format!("Hậu tố không chứa đủ các số yêu cầu ({})", config.suffix_value) }).unwrap();
        }
    }
    if config.toggle_blacklist_filter && !config.blacklist_digits.is_empty() {
        let forbidden_digits: HashSet<u8> = config.blacklist_digits
            .split(',')
            .filter_map(|s| s.trim().parse::<u8>().ok())
            .collect();
        if !forbidden_digits.is_empty() && phone.original_digits.iter().any(|d| forbidden_digits.contains(d)) {
            return serde_wasm_bindgen::to_value(&CheckResult::Invalid { reason: format!("Chứa số bị cấm ({})", config.blacklist_digits) }).unwrap();
        }
    }

    // 3. Apply Static Balance Filter
    if config.toggle_static_balance && !phone.has_static_balance() {
        return serde_wasm_bindgen::to_value(&CheckResult::Invalid { reason: "Không qua bộ lọc Chẵn/Lẻ & Tổng 8.".to_string() }).unwrap();
    }

    // 4. Apply mode-specific logic
    match config.mode {
        AnalysisMode::Compatibility => {
            let t_digits = phone.get_transformed_digits();
            
            let mut counts = [0; 5];
            t_digits.iter().for_each(|&d| counts[digit_to_element(d) as usize] += 1);

            if config.toggle_completeness && counts.iter().any(|&c| c == 0) {
                return serde_wasm_bindgen::to_value(&CheckResult::Invalid { reason: "Không đủ 5 hành.".to_string() }).unwrap();
            }
            if counts.iter().any(|&c| c > config.filter_any_max) {
                return serde_wasm_bindgen::to_value(&CheckResult::Invalid { reason: format!("Một hành xuất hiện quá {} lần.", config.filter_any_max) }).unwrap();
            }

            let (sinh, cung, bi_khac, khac, _) = get_element_roles(config.user_menh);
            if counts[khac as usize] > config.filter_khac_max {
                return serde_wasm_bindgen::to_value(&CheckResult::Invalid { reason: "Vượt quá số lượng Hành Khắc Mệnh.".to_string() }).unwrap();
            }
            if counts[bi_khac as usize] > config.filter_bi_khac_max {
                return serde_wasm_bindgen::to_value(&CheckResult::Invalid { reason: "Vượt quá số lượng Hành Bị Khắc.".to_string() }).unwrap();
            }
            if counts[sinh as usize] < config.filter_sinh_min {
                return serde_wasm_bindgen::to_value(&CheckResult::Invalid { reason: "Không đủ số lượng Hành Sinh Mệnh.".to_string() }).unwrap();
            }
            if counts[cung as usize] < config.filter_cung_min {
                return serde_wasm_bindgen::to_value(&CheckResult::Invalid { reason: "Không đủ số lượng Hành Cùng Mệnh.".to_string() }).unwrap();
            }
            if (counts[sinh as usize] + counts[cung as usize]) > config.filter_tong_max {
                return serde_wasm_bindgen::to_value(&CheckResult::Invalid { reason: "Tổng (Sinh + Cùng) quá lớn.".to_string() }).unwrap();
            }

            let score_adj = phone.calculate_adjacent_score(&t_digits);
            let score_comp = phone.calculate_compatibility_score(&t_digits, &config);
            let final_score = (score_adj * 0.4) + (score_comp * 0.6);
            serde_wasm_bindgen::to_value(&CheckResult::Valid { score: final_score }).unwrap()
        }
        AnalysisMode::AbsoluteBalance => {
            let t_digits = phone.get_transformed_digits();
            if !phone.has_absolute_balance(&t_digits) {
                return serde_wasm_bindgen::to_value(&CheckResult::Invalid { reason: "Không đạt Cân bằng tuyệt đối (2 số mỗi hành).".to_string() }).unwrap();
            }

            let score_adj = phone.calculate_adjacent_score(&t_digits);
            serde_wasm_bindgen::to_value(&CheckResult::Valid { score: score_adj }).unwrap()
        }
    }
}

impl Default for Element { fn default() -> Self { Element::Kim } }
