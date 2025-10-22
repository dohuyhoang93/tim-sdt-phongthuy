// Import các hàm từ gói WASM đã được biên dịch
import init, { analyze, quick_check_single_number } from './pkg/calsdt.js';

let analysisResults = []; // Store results globally

async function main() {
    // Khởi tạo module WebAssembly
    await init();

    // Lấy các element từ DOM
    const modeRadios = document.querySelectorAll('input[name="analysisMode"]');
    const compatibilityOptions = document.getElementById('compatibility-options');
    const compatibilitySettings = document.getElementById('compatibility-settings');
    const analyzeButton = document.getElementById('analyze-button');
    const spinner = document.getElementById('spinner');
    const statusArea = document.getElementById('status-area');
    const fileInput = document.getElementById('file-input');
    const resultsCount = document.getElementById('results-count');
    const downloadButton = document.getElementById('download-button');
    const quickCheckButton = document.getElementById('quick-check-button');
    const quickCheckInput = document.getElementById('quick-check-input');
    const quickCheckResult = document.getElementById('quick-check-result');

    // Xử lý việc thay đổi chế độ
    modeRadios.forEach(radio => {
        radio.addEventListener('change', () => {
            const isCompatibility = document.getElementById('mode-compatibility').checked;
            compatibilityOptions.style.display = isCompatibility ? 'block' : 'none';
            compatibilitySettings.style.display = isCompatibility ? 'block' : 'none';
        });
    });

    // Xử lý sự kiện bấm nút Phân tích
    analyzeButton.addEventListener('click', async () => {
        if (!fileInput.files || fileInput.files.length === 0) {
            alert('Vui lòng chọn một file để phân tích.');
            return;
        }

        // Hiển thị trạng thái đang tải
        spinner.style.display = 'inline-block';
        analyzeButton.disabled = true;
        statusArea.classList.add('d-none');

        try {
            const fileContent = await readFileContent(fileInput.files[0]);
            const config = buildConfigFromUI();
            
            console.log("Sending config to WASM:", config); // DEBUGGING STEP

            // Gọi hàm `analyze` từ WASM
            analysisResults = analyze(fileContent, config); // Store results, don't download

            // Hiển thị kết quả trong bảng
            displayResultsInTable(analysisResults);

            // Cập nhật và hiển thị trạng thái
            resultsCount.textContent = analysisResults.length;
            statusArea.classList.remove('d-none');

        } catch (error) {
            console.error("Đã có lỗi xảy ra trong quá trình phân tích:", error);
            alert("Đã có lỗi xảy ra. Vui lòng kiểm tra console (F12) để biết thêm chi tiết.");
        }

        // Hoàn tất, trả lại trạng thái ban đầu cho nút bấm
        spinner.style.display = 'none';
        analyzeButton.disabled = false;
    });

    // Add event listener for the new download button
    downloadButton.addEventListener('click', () => {
        if (analysisResults && analysisResults.length > 0) {
            const outputContent = formatResults(analysisResults);
            triggerDownload(outputContent);
        } else {
            alert('Không có kết quả để tải xuống.');
        }
    });

    // Add event listener for the new Quick Check button
    quickCheckButton.addEventListener('click', () => {
        const numberToCheck = quickCheckInput.value;
        if (!numberToCheck || numberToCheck.trim() === '') {
            quickCheckResult.textContent = 'Vui lòng nhập một số.';
            quickCheckResult.className = 'fw-bold text-warning';
            return;
        }

        try {
            const config = buildConfigFromUI();
            const result_obj = quick_check_single_number(numberToCheck, config);

            if (result_obj.Valid) {
                const score = result_obj.Valid.score.toFixed(2);
                quickCheckResult.textContent = `✅ Hợp lệ. Điểm: ${score}`;
                quickCheckResult.className = 'fw-bold text-success';
            } else if (result_obj.Invalid) {
                quickCheckResult.textContent = `❌ Không hợp lệ: ${result_obj.Invalid.reason}`;
                quickCheckResult.className = 'fw-bold text-danger';
            }
        } catch (error) {
            console.error("Lỗi khi kiểm tra nhanh:", error);
            quickCheckResult.textContent = 'Đã xảy ra lỗi khi kiểm tra.';
            quickCheckResult.className = 'fw-bold text-danger';
        }
    });

    // Initialize Bootstrap tooltips
    const tooltipTriggerList = document.querySelectorAll('[data-bs-toggle="tooltip"]');
    [...tooltipTriggerList].map(tooltipTriggerEl => new bootstrap.Tooltip(tooltipTriggerEl));
}

// Hàm đọc nội dung file (bất đồng bộ)
function readFileContent(file) {
    return new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = (event) => resolve(event.target.result);
        reader.onerror = (error) => reject(error);
        reader.readAsText(file);
    });
}

// Hàm thu thập tất cả cấu hình từ UI
function buildConfigFromUI() {
    const isCompatibility = document.getElementById('mode-compatibility').checked;
    
    return {
        mode: isCompatibility ? 'Compatibility' : 'AbsoluteBalance',
        user_menh: document.getElementById('user-menh').value,
        
        // Score weights
        score_sinh: parseFloat(document.getElementById('score-sinh').value),
        score_cung: parseFloat(document.getElementById('score-cung').value),
        score_bi_khac: parseFloat(document.getElementById('score-bi-khac').value),
        score_sinh_xuat: parseFloat(document.getElementById('score-sinh-xuat').value),
        score_khac: parseFloat(document.getElementById('score-khac').value),

        // Filter thresholds
        filter_khac_max: parseInt(document.getElementById('filter-khac-max').value),
        filter_bi_khac_max: parseInt(document.getElementById('filter-bi-khac-max').value),
        filter_sinh_min: parseInt(document.getElementById('filter-sinh-min').value),
        filter_cung_min: parseInt(document.getElementById('filter-cung-min').value),
        filter_tong_max: parseInt(document.getElementById('filter-tong-max').value),
        filter_any_max: parseInt(document.getElementById('filter-any-max').value),

        // Toggles
        toggle_static_balance: document.getElementById('toggle-static-balance').checked,
        toggle_completeness: document.getElementById('toggle-completeness').checked,

        // Custom Filters
        toggle_prefix_filter: document.getElementById('toggle-prefix-filter').checked,
        prefix_value: document.getElementById('input-prefix').value,
        toggle_suffix_filter: document.getElementById('toggle-suffix-filter').checked,
        suffix_value: document.getElementById('input-suffix').value,
        toggle_blacklist_filter: document.getElementById('toggle-blacklist-filter').checked,
        blacklist_digits: document.getElementById('input-blacklist').value,
    };
}

// Hàm hiển thị kết quả trong bảng
function displayResultsInTable(results) {
    const tableBody = document.getElementById('results-table-body');
    const displayArea = document.getElementById('results-display-area');

    // Clear previous results
    tableBody.innerHTML = '';

    if (results.length === 0) {
        displayArea.classList.add('d-none');
        return;
    }

    // Populate new results
    results.forEach((result, index) => {
        const row = document.createElement('tr');
        row.innerHTML = `
            <th scope="row">${index + 1}</th>
            <td>${result.number}</td>
            <td>${result.score.toFixed(2)}</td>
        `;
        tableBody.appendChild(row);
    });

    // Show the results area
    displayArea.classList.remove('d-none');
}

// Hàm định dạng kết quả để tải về
function formatResults(results) {
    return results.map(r => `${r.number}  score=${r.score.toFixed(2)}`).join('\n');
}

// Hàm kích hoạt việc tải file
function triggerDownload(content) {
    const now = new Date();
    const timestamp = `${now.getFullYear()}${(now.getMonth() + 1).toString().padStart(2, '0')}${now.getDate().toString().padStart(2, '0')}_${now.getHours().toString().padStart(2, '0')}${now.getMinutes().toString().padStart(2, '0')}${now.getSeconds().toString().padStart(2, '0')}`;
    const filename = `ket_qua_${timestamp}.txt`;

    const blob = new Blob([content], { type: 'text/plain;charset=utf-8' });
    const link = document.createElement('a');
    link.href = URL.createObjectURL(blob);
    link.download = filename;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
}

main().catch(console.error);