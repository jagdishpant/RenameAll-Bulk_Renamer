# ⚡ RenameAll - Bulk Renamer: User Guide & Examples

RenameAll is a lightweight, smooth and blazing-fast batch file renaming utility for inspired by Microsoft PowerToys PowerRename, written in pure **Rust**.

---

## 📌 Quick Interface Overview

| UI Control | What it does |
| :--- | :--- |
| **Search:** | The pattern or text to locate in each filename. |
| **Replace with:** | The replacement string, regex capture group ('$1', '$2'), or counter/date tokens. |
| **Use Regular Expressions** | Enables full Regular Expression syntax (e.g. '^', '$', '\d+', '(.*)'). |
| **Match All Occurrences** | Replaces every instance (global replace) rather than just the first occurrence. |
| **Case Sensitive** | Distinguishes between uppercase and lowercase when searching. |
| **Apply To** | Choose between **Filename only** (preserves '.ext'), **Filename + Extension**, or **Extension only**. |
| **Sequential Numbering** | Injects counters ('01', '001') as prefix, suffix, or via '${count}' token. |
| **Text Formatting** | Convert case on the fly: 'lowercase', 'UPPERCASE', 'Title Case', or 'Capitalize'. |

---

## 📚 Practical Recipe Guide

### 1. Add a Prefix to Filenames
Prepend a word, tag, or date to the beginning of files without affecting their extensions.

- **Example Goal:** 'report.pdf' $\to$ '2026_Q4_report.pdf'
- **Method A: Using Regular Expressions (Recommended)**
  - **Search:** '^' *(The '^' symbol represents the beginning of the text)*
  - **Replace with:** '2026_Q4_'
  - **Use Regular Expressions:** ☑ **Checked**
  - **Apply To:** 'Filename only'
- **Method B: Using Sequential Numbering Counter**
  - Enable **Item Counter**
  - **Position:** 'Prefix'
  - **Separator:** '_'
  - **Padding digits:** '2' (produces '01_report.pdf', '02_invoice.pdf', etc.)

---

### 2. Add a Suffix to Filenames
Append a status, version, or label to the end of the filename (before the file extension).

- **Example Goal:** 'document.docx' $\to$ 'document_FINAL.docx'
- **Method A: Using Regular Expressions (Recommended)**
  - **Search:** '$' *(The '$' symbol represents the end of the text)*
  - **Replace with:** '_FINAL'
  - **Use Regular Expressions:** ☑ **Checked**
  - **Apply To:** 'Filename only'
- **Method B: Using Sequential Numbering Counter**
  - Enable **Item Counter**
  - **Position:** 'Suffix'
  - **Separator:** '_'
  - **Padding digits:** '3' (produces 'document_001.docx', 'document_002.docx', etc.)

---

### 3. Replace All File Names with New Numbered Names
Reset irregular or messy filenames (like camera or scanner outputs) into a clean, uniform numbered series.

- **Example Goal:** 
  - 'IMG_9041.JPG' $\to$ 'Vacation_001.JPG'
  - 'DSC_0012.JPG' $\to$ 'Vacation_002.JPG'
  - 'scan_abc.JPG' $\to$ 'Vacation_003.JPG'
- **Configuration:**
  - **Search:** '.*' *(Matches the entire existing filename)*
  - **Replace with:** 'Vacation_${count}'
  - **Use Regular Expressions:** ☑ **Checked**
  - **Apply To:** 'Filename only' *(Leaves '.JPG' unchanged)*
  - **Item Counter:**
    - Enable **Item Counter**: ☑ **Checked**
    - **Start:** '1'
    - **Step:** '1'
    - **Padding digits:** '3' *(formats 1 $\to$ '001', 42 $\to$ '042')*

> [!TIP]
> You can also insert the current year or date directly using tokens:
> 'Replace with: Vacation_${YYYY}_${count}' $\to$ 'Vacation_2026_001.JPG'

---

### 4. Replace Filename Before a Particular Character
Target and update or remove metadata prefixes that precede a delimiter (like '-' or '_').

#### Case 4A: Replace the prefix before a hyphen
- **Example Goal:** 'OLD_DEP-Budget2026.xlsx' $\to$ 'FINANCE-Budget2026.xlsx'
- **Configuration:**
  - **Search:** '^[^-]*-' *(Matches from start up to the first hyphen)*
  - **Replace with:** 'FINANCE-'
  - **Use Regular Expressions:** ☑ **Checked**
  - **Apply To:** 'Filename only'

#### Case 4B: Strip/Remove everything before the hyphen
- **Example Goal:** 'OLD_DEP-Budget2026.xlsx' $\to$ 'Budget2026.xlsx'
- **Configuration:**
  - **Search:** '^[^-]*-'
  - **Replace with:** *(Leave empty)*
  - **Use Regular Expressions:** ☑ **Checked**
  - **Apply To:** 'Filename only'

---

### 5. Replace Filename After a Particular Character
Target and update or truncate suffixes following a delimiter (e.g., '-' or '_').

#### Case 5A: Replace the suffix after a hyphen
- **Example Goal:** 'Report-Draft_v1.pdf' $\to$ 'Report-Approved.pdf'
- **Configuration:**
  - **Search:** '-[^-]*$' *(Matches the hyphen and all characters until the end)*
  - **Replace with:** '-Approved'
  - **Use Regular Expressions:** ☑ **Checked**
  - **Apply To:** 'Filename only'

#### Case 5B: Strip/Remove everything after the hyphen
- **Example Goal:** 'ProjectName-temp_copy.zip' $\to$ 'ProjectName.zip'
- **Configuration:**
  - **Search:** '-[^-]*$'
  - **Replace with:** *(Leave empty)*
  - **Use Regular Expressions:** ☑ **Checked**
  - **Apply To:** 'Filename only'

---

### 6. Change Character Positions (Reorder Segments)
Rearrange structured filename components (such as division codes, employee IDs, and names) using Regex capture groups.

- **Example Goal:** 'DIV-EMP-Name.txt' $\to$ 'EMP-DIV-Name.txt'
  *(e.g., 'HR-1042-JohnDoe.txt' $\to$ '1042-HR-JohnDoe.txt')*

- **Configuration:**
  - **Search:** '^([^-]+)-([^-]+)-(.*)$'
  - **Replace with:** '$2-$1-$3'
  - **Use Regular Expressions:** ☑ **Checked**
  - **Apply To:** 'Filename only'

#### 🔍 How this pattern works:
- '^' : Start of filename.
- '([^-]+)' **(Group 1)**: Matches 'DIV' (any characters before the first '-').
- '-' : Matches the literal first hyphen.
- '([^-]+)' **(Group 2)**: Matches 'EMP' (any characters between the first and second '-').
- '-' : Matches the literal second hyphen.
- '(.*)$' **(Group 3)**: Matches 'Name' (the remainder of the filename).
- '$2-$1-$3': Swaps Group 1 and Group 2 while leaving Group 3 in place!

---

## 🛠️ Summary Reference Cheat Sheet

| Task | Search | Replace with | Regex? | Apply To |
| :--- | :--- | :--- | :---: | :--- |
| **1. Add Prefix** | '^' | 'MyPrefix_' | Yes | Filename only |
| **2. Add Suffix** | '$' | '_MySuffix' | Yes | Filename only |
| **3. Clean Numbering** | '.*' | 'Item_${count}' | Yes | Filename only |
| **4. Replace Before '-'** | '^[^-]*-' | 'NewPrefix-' | Yes | Filename only |
| **5. Replace After '-'** | '-[^-]*$' | '-NewSuffix' | Yes | Filename only |
| **6. Swap Segments ('A-B-C')** | '^([^-]+)-([^-]+)-(.*)$' | '$2-$1-$3' | Yes | Filename only |
| **Batch Extension Change** | '.*' | 'jpg' | No | Extension only |
