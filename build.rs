//! Build script for Fresh editor
//!
//! Generates TypeScript type definitions from Rust op definitions

use std::collections::HashMap;
use std::fs;
use std::path::Path;

fn main() {
    // Only rerun if ts_runtime.rs changes
    println!("cargo::rerun-if-changed=src/ts_runtime.rs");

    if let Err(e) = generate_typescript_types() {
        eprintln!("Warning: Failed to generate TypeScript types: {}", e);
    }
}

/// Information about a single op
struct OpInfo {
    js_name: String,
    params: Vec<ParamInfo>,
    return_type: String,
    is_async: bool,
    doc_comment: String,
}

/// Information about a parameter
struct ParamInfo {
    name: String,
    ts_type: String,
    is_optional: bool,
}

/// Information about a struct to export as interface
struct StructInfo {
    name: String,
    ts_name: String,
    fields: Vec<FieldInfo>,
    doc_comment: String,
}

/// Information about a struct field
struct FieldInfo {
    name: String,
    ts_type: String,
    is_optional: bool,
    doc_comment: String,
}

/// Parse Rust type to TypeScript type
fn rust_type_to_ts(rust_type: &str) -> String {
    let rust_type = rust_type.trim();

    // Handle Option<T>
    if rust_type.starts_with("Option<") && rust_type.ends_with('>') {
        let inner = &rust_type[7..rust_type.len() - 1];
        return format!("{} | null", rust_type_to_ts(inner));
    }

    // Handle Result<T, E> - in JS, errors throw, so we just return T
    if rust_type.starts_with("Result<") && rust_type.ends_with('>') {
        let inner = &rust_type[7..rust_type.len() - 1];
        // Find the first comma at depth 0
        let mut depth = 0;
        let mut comma_pos = None;
        for (i, ch) in inner.chars().enumerate() {
            match ch {
                '<' => depth += 1,
                '>' => depth -= 1,
                ',' if depth == 0 => {
                    comma_pos = Some(i);
                    break;
                }
                _ => {}
            }
        }
        let ok_type = if let Some(pos) = comma_pos {
            &inner[..pos]
        } else {
            inner
        };
        return rust_type_to_ts(ok_type.trim());
    }

    // Handle Vec<T>
    if rust_type.starts_with("Vec<") && rust_type.ends_with('>') {
        let inner = &rust_type[4..rust_type.len() - 1];
        return format!("{}[]", rust_type_to_ts(inner));
    }

    // Handle HashMap
    if rust_type.starts_with("HashMap<") || rust_type.starts_with("std::collections::HashMap<") {
        return "Record<string, unknown>".to_string();
    }

    // Handle tuples like (String, String)
    if rust_type.starts_with('(') && rust_type.ends_with(')') {
        let inner = &rust_type[1..rust_type.len() - 1];
        let parts: Vec<&str> = inner.split(',').collect();
        let ts_parts: Vec<String> = parts.iter().map(|p| rust_type_to_ts(p.trim())).collect();
        return format!("[{}]", ts_parts.join(", "));
    }

    match rust_type {
        // Primitives
        "u32" | "u8" | "usize" | "i32" | "i64" | "u64" | "f32" | "f64" => "number".to_string(),
        "bool" => "boolean".to_string(),
        "String" | "&str" => "string".to_string(),
        "()" => "void".to_string(),

        // Known custom types - map to their TypeScript interface names
        "SpawnResult" => "SpawnResult".to_string(),
        "FileStat" => "FileStat".to_string(),
        "TsBufferInfo" => "BufferInfo".to_string(),
        "TsCursorInfo" => "CursorInfo".to_string(),
        "TsViewportInfo" => "ViewportInfo".to_string(),
        "TsSelectionRange" => "SelectionRange".to_string(),
        "TsSuggestion" => "PromptSuggestion".to_string(),
        "DirEntry" => "DirEntry".to_string(),
        "CreateVirtualBufferOptions" => "CreateVirtualBufferOptions".to_string(),
        "CreateVirtualBufferInExistingSplitOptions" => "CreateVirtualBufferInExistingSplitOptions".to_string(),
        "TsTextPropertyEntry" => "TextPropertyEntry".to_string(),

        // Serde JSON value
        "serde_json::Value" => "unknown".to_string(),

        _ => rust_type.to_string(),
    }
}

/// Convert op_fresh_xxx to camelCase
fn op_name_to_js(op_name: &str) -> String {
    let name = op_name.strip_prefix("op_fresh_").unwrap_or(op_name);
    let parts: Vec<&str> = name.split('_').collect();
    if parts.is_empty() {
        return name.to_string();
    }

    let mut result = parts[0].to_string();
    for part in &parts[1..] {
        if !part.is_empty() {
            let mut chars = part.chars();
            if let Some(first) = chars.next() {
                result.push(first.to_ascii_uppercase());
                result.extend(chars);
            }
        }
    }
    result
}

/// Extract doc comments before a given line index
fn extract_doc_comments(lines: &[&str], target_line: usize) -> String {
    let mut docs = Vec::new();
    let mut i = target_line.saturating_sub(1);

    loop {
        if i == 0 && !lines[0].trim().starts_with("///") {
            break;
        }

        let line = lines[i].trim();
        if line.starts_with("///") {
            let doc = line.strip_prefix("///").unwrap_or("").trim_start();
            docs.push(doc.to_string());
        } else if line.starts_with("#[") || line.is_empty() {
            // Skip attributes and empty lines
            if i == 0 {
                break;
            }
            i -= 1;
            continue;
        } else {
            break;
        }

        if i == 0 {
            break;
        }
        i -= 1;
    }

    docs.reverse();
    docs.join("\n")
}

/// Extract op definitions from Rust source
fn extract_ops(rust_source: &str) -> Vec<OpInfo> {
    let mut ops = Vec::new();
    let lines: Vec<&str> = rust_source.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Look for #[op2...] attribute
        if line.starts_with("#[op2") {
            let is_async = line.contains("async");

            // Check for #[string] or #[serde] return marker on following lines
            let mut has_string_return = false;
            let mut has_serde_return = false;
            let mut fn_line_idx = i + 1;

            while fn_line_idx < lines.len() {
                let next_line = lines[fn_line_idx].trim();
                if next_line.starts_with("#[string]") {
                    has_string_return = true;
                    fn_line_idx += 1;
                } else if next_line.starts_with("#[serde]") {
                    has_serde_return = true;
                    fn_line_idx += 1;
                } else if next_line.starts_with("#[allow") {
                    fn_line_idx += 1;
                } else if next_line.starts_with("fn ") || next_line.starts_with("async fn ") {
                    break;
                } else if next_line.is_empty() || next_line.starts_with("//") {
                    fn_line_idx += 1;
                } else {
                    break;
                }
            }

            // Parse function signature
            if fn_line_idx < lines.len() {
                let fn_line = lines[fn_line_idx].trim();
                if fn_line.contains("op_fresh_") {
                    // Extract doc comments
                    let doc_comment = extract_doc_comments(&lines, i);

                    if let Some(mut op_info) = parse_fn_signature(fn_line, has_string_return, has_serde_return, is_async, &lines[fn_line_idx..]) {
                        op_info.doc_comment = doc_comment;
                        ops.push(op_info);
                    }
                }
            }
        }
        i += 1;
    }

    ops
}

/// Parse a function signature to extract op info
fn parse_fn_signature(line: &str, has_string_return: bool, has_serde_return: bool, is_async: bool, remaining_lines: &[&str]) -> Option<OpInfo> {
    // Extract function name
    let fn_keyword = if line.contains("async fn ") { "async fn " } else { "fn " };
    let fn_start = line.find(fn_keyword)? + fn_keyword.len();
    let paren_start = line.find('(')?;
    let fn_name = &line[fn_start..paren_start];

    if !fn_name.starts_with("op_fresh_") {
        return None;
    }

    let js_name = op_name_to_js(fn_name);

    // Find the full parameter list (may span multiple lines)
    let mut full_sig = String::new();
    for l in remaining_lines {
        full_sig.push_str(l.trim());
        full_sig.push(' ');
        if l.contains('{') || (l.contains(')') && (l.contains("->") || l.trim().ends_with('{'))) {
            break;
        }
    }

    // Extract parameters between ( and )
    let params_start = full_sig.find('(')? + 1;
    let params_end = full_sig.find(')')?;
    let params_str = &full_sig[params_start..params_end];

    // Parse parameters
    let mut params = Vec::new();
    let mut depth = 0;
    let mut current = String::new();

    for ch in params_str.chars() {
        match ch {
            '<' | '[' | '(' => {
                depth += 1;
                current.push(ch);
            }
            '>' | ']' | ')' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                if !current.trim().is_empty() {
                    if let Some(param) = parse_param(current.trim()) {
                        params.push(param);
                    }
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        if let Some(param) = parse_param(current.trim()) {
            params.push(param);
        }
    }

    // Extract return type
    let return_type = if has_string_return {
        "string".to_string()
    } else if let Some(arrow_pos) = full_sig.find("->") {
        let ret_start = arrow_pos + 2;
        let ret_end = full_sig[ret_start..].find('{').map(|p| ret_start + p).unwrap_or(full_sig.len());
        let rust_ret = full_sig[ret_start..ret_end].trim();

        // For serde return, the type is already the Rust type
        if has_serde_return || rust_ret.starts_with("Result<") || rust_ret.starts_with("Option<") || rust_ret.starts_with("Vec<") {
            rust_type_to_ts(rust_ret)
        } else {
            rust_type_to_ts(rust_ret)
        }
    } else {
        "void".to_string()
    };

    Some(OpInfo {
        js_name,
        params,
        return_type,
        is_async,
        doc_comment: String::new(),
    })
}

/// Parse a single parameter
fn parse_param(param_str: &str) -> Option<ParamInfo> {
    let param_str = param_str.trim();

    // Skip state parameter
    if param_str.contains("OpState") || param_str.starts_with("state:") || param_str.starts_with("state ") {
        return None;
    }

    // Skip Rc<RefCell<OpState>>
    if param_str.contains("Rc<RefCell<OpState>>") {
        return None;
    }

    // Check for #[string] or #[serde] attribute
    let is_string = param_str.contains("#[string]");
    let is_serde = param_str.contains("#[serde]");
    let clean_param = param_str
        .replace("#[string]", "")
        .replace("#[serde]", "")
        .trim()
        .to_string();

    // Parse name: type
    let parts: Vec<&str> = clean_param.splitn(2, ':').collect();
    if parts.len() != 2 {
        return None;
    }

    let name = parts[0].trim().to_string();
    let rust_type = parts[1].trim();

    // Check if the type is Option<T>
    let is_optional = rust_type.starts_with("Option<");

    let ts_type = if is_string {
        // #[string] can be on Option<String> too
        if is_optional {
            "string | null".to_string()
        } else {
            "string".to_string()
        }
    } else if is_serde {
        rust_type_to_ts(rust_type)
    } else {
        rust_type_to_ts(rust_type)
    };

    Some(ParamInfo {
        name,
        ts_type,
        is_optional,
    })
}

/// Extract struct definitions that should become TypeScript interfaces
fn extract_structs(rust_source: &str) -> Vec<StructInfo> {
    let mut structs = Vec::new();
    let lines: Vec<&str> = rust_source.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Look for #[derive(...Serialize...)] or #[derive(...Deserialize...)]
        if line.starts_with("#[derive(") && (line.contains("Serialize") || line.contains("Deserialize")) {
            // Find the struct definition
            let mut struct_line_idx = i + 1;
            while struct_line_idx < lines.len() {
                let next_line = lines[struct_line_idx].trim();
                if next_line.starts_with("struct ") {
                    break;
                } else if next_line.starts_with("#[") || next_line.is_empty() {
                    struct_line_idx += 1;
                } else {
                    break;
                }
            }

            if struct_line_idx < lines.len() && lines[struct_line_idx].trim().starts_with("struct ") {
                let doc_comment = extract_doc_comments(&lines, i);

                if let Some(mut struct_info) = parse_struct(&lines, struct_line_idx) {
                    struct_info.doc_comment = doc_comment;
                    structs.push(struct_info);
                }
            }
        }
        i += 1;
    }

    structs
}

/// Parse a struct definition
fn parse_struct(lines: &[&str], struct_line_idx: usize) -> Option<StructInfo> {
    let struct_line = lines[struct_line_idx].trim();

    // Extract struct name
    let name_start = struct_line.find("struct ")? + 7;
    let name_end = struct_line[name_start..].find(|c: char| c == ' ' || c == '{')
        .map(|p| name_start + p)
        .unwrap_or(struct_line.len());
    let name = struct_line[name_start..name_end].trim().to_string();

    // Map Rust struct names to TypeScript interface names
    let ts_name = match name.as_str() {
        "TsBufferInfo" => "BufferInfo".to_string(),
        "TsCursorInfo" => "CursorInfo".to_string(),
        "TsViewportInfo" => "ViewportInfo".to_string(),
        "TsSelectionRange" => "SelectionRange".to_string(),
        "TsSuggestion" => "PromptSuggestion".to_string(),
        "TsTextPropertyEntry" => "TextPropertyEntry".to_string(),
        _ => name.clone(),
    };

    // Find fields (between { and })
    let mut fields = Vec::new();
    let mut in_struct = false;
    let mut field_doc = String::new();

    for j in struct_line_idx..lines.len() {
        let line = lines[j].trim();

        if line.contains('{') {
            in_struct = true;
            continue;
        }

        if !in_struct {
            continue;
        }

        if line.contains('}') {
            break;
        }

        // Collect doc comments for fields
        if line.starts_with("///") {
            let doc = line.strip_prefix("///").unwrap_or("").trim_start();
            if !field_doc.is_empty() {
                field_doc.push('\n');
            }
            field_doc.push_str(doc);
            continue;
        }

        // Skip empty lines and attributes
        if line.is_empty() || line.starts_with("#[") {
            continue;
        }

        // Parse field: name: Type,
        if let Some(field) = parse_struct_field(line, &field_doc) {
            fields.push(field);
        }
        field_doc.clear();
    }

    Some(StructInfo {
        name,
        ts_name,
        fields,
        doc_comment: String::new(),
    })
}

/// Parse a struct field
fn parse_struct_field(line: &str, doc_comment: &str) -> Option<FieldInfo> {
    let line = line.trim().trim_end_matches(',');

    let parts: Vec<&str> = line.splitn(2, ':').collect();
    if parts.len() != 2 {
        return None;
    }

    let name = parts[0].trim().to_string();
    let rust_type = parts[1].trim();

    let is_optional = rust_type.starts_with("Option<");
    let ts_type = rust_type_to_ts(rust_type);

    Some(FieldInfo {
        name,
        ts_type,
        is_optional,
        doc_comment: doc_comment.to_string(),
    })
}

/// Format a doc comment as JSDoc
fn format_jsdoc(doc: &str, indent: &str) -> String {
    if doc.is_empty() {
        return String::new();
    }

    let lines: Vec<&str> = doc.lines().collect();
    if lines.len() == 1 && !lines[0].contains('@') {
        return format!("{}/** {} */\n", indent, lines[0]);
    }

    let mut result = format!("{}/**\n", indent);
    for line in lines {
        if line.is_empty() {
            result.push_str(&format!("{} *\n", indent));
        } else {
            result.push_str(&format!("{} * {}\n", indent, line));
        }
    }
    result.push_str(&format!("{} */\n", indent));
    result
}

/// Generate the TypeScript definition file
fn generate_typescript_types() -> Result<(), Box<dyn std::error::Error>> {
    let rust_source = fs::read_to_string("src/ts_runtime.rs")?;
    let ops = extract_ops(&rust_source);
    let structs = extract_structs(&rust_source);

    // Categorize ops
    let mut categories: HashMap<&str, Vec<&OpInfo>> = HashMap::new();
    categories.insert("status", Vec::new());
    categories.insert("query", Vec::new());
    categories.insert("buffer_info", Vec::new());
    categories.insert("prompt", Vec::new());
    categories.insert("mutation", Vec::new());
    categories.insert("async", Vec::new());
    categories.insert("overlay", Vec::new());
    categories.insert("filesystem", Vec::new());
    categories.insert("environment", Vec::new());
    categories.insert("path", Vec::new());
    categories.insert("event", Vec::new());
    categories.insert("virtual_buffer", Vec::new());

    for op in &ops {
        let category = categorize_op(&op.js_name, op.is_async);
        categories.get_mut(category).unwrap().push(op);
    }

    // Generate TypeScript
    let mut output = String::new();
    output.push_str(
        r#"/**
 * Fresh Editor TypeScript Plugin API
 *
 * AUTO-GENERATED FILE - DO NOT EDIT MANUALLY
 * Generated from src/ts_runtime.rs by build.rs
 *
 * This file provides type definitions for the Fresh editor's TypeScript plugin system.
 * Plugins have access to the global `editor` object which provides methods to:
 * - Query editor state (buffers, cursors, viewports)
 * - Modify buffer content (insert, delete text)
 * - Add visual decorations (overlays, highlighting)
 * - Interact with the editor UI (status messages, prompts)
 */

declare global {
  /**
   * Global editor API object available to all TypeScript plugins
   */
  const editor: EditorAPI;
}

/**
 * Buffer identifier (unique numeric ID)
 */
type BufferId = number;

"#,
    );

    // Add interface definitions from structs
    for struct_info in &structs {
        // Skip internal structs
        if struct_info.name == "TsRuntimeState" {
            continue;
        }

        output.push_str(&format_jsdoc(&struct_info.doc_comment, ""));
        output.push_str(&format!("interface {} {{\n", struct_info.ts_name));

        for field in &struct_info.fields {
            if !field.doc_comment.is_empty() {
                output.push_str(&format_jsdoc(&field.doc_comment, "  "));
            }

            let optional_marker = if field.is_optional { "?" } else { "" };
            output.push_str(&format!("  {}{}: {};\n", field.name, optional_marker, field.ts_type));
        }

        output.push_str("}\n\n");
    }

    // Start EditorAPI interface
    output.push_str(
        r#"/**
 * Main editor API interface
 */
interface EditorAPI {
"#,
    );

    // Add ops by category
    add_category_ops(&mut output, "Status and Logging", &categories["status"]);
    add_category_ops(&mut output, "Buffer Queries", &categories["query"]);
    add_category_ops(&mut output, "Buffer Info Queries", &categories["buffer_info"]);
    add_category_ops(&mut output, "Prompt Operations", &categories["prompt"]);
    add_category_ops(&mut output, "Buffer Mutations", &categories["mutation"]);
    add_category_ops(&mut output, "Async Operations", &categories["async"]);
    add_category_ops(&mut output, "Overlay Operations", &categories["overlay"]);
    add_category_ops(&mut output, "File System Operations", &categories["filesystem"]);
    add_category_ops(&mut output, "Environment Operations", &categories["environment"]);
    add_category_ops(&mut output, "Path Operations", &categories["path"]);
    add_category_ops(&mut output, "Event/Hook Operations", &categories["event"]);
    add_category_ops(&mut output, "Virtual Buffer Operations", &categories["virtual_buffer"]);

    output.push_str(
        r#"}

// Export for module compatibility
export {};
"#,
    );

    // Ensure types directory exists
    let types_dir = Path::new("types");
    if !types_dir.exists() {
        fs::create_dir_all(types_dir)?;
    }

    // Write output
    fs::write("types/fresh.d.ts", output)?;

    println!("cargo::warning=Generated types/fresh.d.ts with {} ops and {} interfaces", ops.len(), structs.len());

    Ok(())
}

/// Categorize an op based on its name
fn categorize_op(js_name: &str, is_async: bool) -> &'static str {
    // Virtual buffer operations
    if js_name.contains("VirtualBuffer") || js_name == "defineMode" || js_name == "showBuffer"
        || js_name == "closeBuffer" || js_name == "focusSplit" || js_name == "setSplitBuffer"
        || js_name == "closeSplit" || js_name == "getTextPropertiesAtCursor" || js_name == "setVirtualBufferContent" {
        return "virtual_buffer";
    }

    // Event operations
    if js_name == "on" || js_name == "off" || js_name == "getHandlers" {
        return "event";
    }

    // Path operations
    if js_name.starts_with("path") {
        return "path";
    }

    // Environment operations
    if js_name == "getEnv" || js_name == "getCwd" {
        return "environment";
    }

    // File system operations
    if js_name == "readFile" || js_name == "writeFile" || js_name == "fileExists"
        || js_name == "fileStat" || js_name == "readDir" {
        return "filesystem";
    }

    // Status ops
    if js_name == "setStatus" || js_name == "debug" {
        return "status";
    }

    // Prompt operations
    if js_name == "startPrompt" || js_name == "setPromptSuggestions" {
        return "prompt";
    }

    // Buffer info queries
    if js_name == "getBufferInfo" || js_name == "listBuffers" || js_name == "getPrimaryCursor"
        || js_name == "getAllCursors" || js_name == "getViewport" {
        return "buffer_info";
    }

    // General queries
    if js_name.starts_with("get") || js_name.starts_with("is") {
        return "query";
    }

    // Overlay operations
    if js_name.contains("Overlay") || js_name.contains("overlay")
        || js_name.contains("VirtualText") || js_name == "refreshLines" {
        return "overlay";
    }

    // Async operations (that aren't already categorized)
    if is_async && js_name == "spawnProcess" {
        return "async";
    }

    // Everything else is mutation
    "mutation"
}

/// Add ops for a category to the output
fn add_category_ops(output: &mut String, category_name: &str, ops: &[&OpInfo]) {
    if ops.is_empty() {
        return;
    }

    output.push_str(&format!("  // === {} ===\n", category_name));

    for op in ops {
        output.push_str(&format_method(op));
    }

    output.push('\n');
}

fn format_method(op: &OpInfo) -> String {
    let mut result = String::new();

    // Add JSDoc if present
    if !op.doc_comment.is_empty() {
        result.push_str(&format_jsdoc(&op.doc_comment, "  "));
    }

    // Format parameters
    let params: Vec<String> = op.params.iter().map(|p| {
        let optional = if p.is_optional { "?" } else { "" };
        format!("{}{}: {}", p.name, optional, p.ts_type)
    }).collect();

    // Format return type (wrap in Promise if async)
    let return_type = if op.is_async {
        format!("Promise<{}>", op.return_type)
    } else {
        op.return_type.clone()
    };

    result.push_str(&format!("  {}({}): {};\n", op.js_name, params.join(", "), return_type));
    result
}
