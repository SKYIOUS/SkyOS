use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use alloc::ffi::CString;
use alloc::vec;

struct ScriptContext {
    vars: Vec<(String, String)>,
    funcs: Vec<(String, Vec<String>)>,
    pos_args: Vec<String>,
    break_flag: bool,
    continue_flag: bool,
}

#[allow(dead_code)]
pub fn run_script(path: &str) -> i64 {
    let content = read_file(path);
    if content.is_empty() { return 1; }
    let mut ctx = ScriptContext { vars: Vec::new(), funcs: Vec::new(), pos_args: Vec::new(), break_flag: false, continue_flag: false };
    execute_lines(&content, &mut ctx)
}

pub fn run_script_with_args(path: &str, args: &[String]) -> i64 {
    let content = read_file(path);
    if content.is_empty() { return 1; }
    let mut ctx = ScriptContext { vars: Vec::new(), funcs: Vec::new(), pos_args: args.to_vec(), break_flag: false, continue_flag: false };
    execute_lines(&content, &mut ctx)
}

fn execute_lines(input: &str, ctx: &mut ScriptContext) -> i64 {
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        if ctx.break_flag || ctx.continue_flag { break; }
        let trimmed = lines[i].trim();
        if trimmed.is_empty() || trimmed.starts_with('#') { i += 1; continue; }

        if trimmed.starts_with("if ") || trimmed == "if" {
            i = execute_if(&lines, i, ctx);
        } else if trimmed.starts_with("for ") {
            let (ni, _) = execute_for(&lines, i, ctx);
            i = ni;
        } else if trimmed.starts_with("while ") || trimmed.starts_with("until ") {
            let (ni, _) = execute_while(&lines, i, ctx);
            i = ni;
        } else if trimmed.starts_with("case ") {
            i = execute_case(&lines, i, ctx);
        } else if trimmed.starts_with("function ") || (trimmed.contains(" ()") && trimmed.trim_end().ends_with("()")) {
            let (ni, _) = define_function(&lines, i, ctx);
            i = ni;
        } else if trimmed == "fi" || trimmed.starts_with("elif ") || trimmed.starts_with("else") || trimmed == "done" || trimmed.starts_with("esac") {
            break;
        } else if trimmed.starts_with("break") {
            ctx.break_flag = true; break;
        } else if trimmed.starts_with("continue") {
            ctx.continue_flag = true; break;
        } else if trimmed.starts_with("return ") {
            break;
        } else {
            execute_line_trimmed(&trimmed, ctx);
            i += 1;
        }
    }
    0
}

fn execute_line_trimmed(line: &str, ctx: &ScriptContext) {
    let expanded = expand_vars(line, ctx);
    let tokens = crate::parser::tokenize(&expanded).unwrap_or_default();
    let pipelines = crate::parser::parse(&tokens);
    crate::executor::execute_pipelines(pipelines);
}

fn execute_if(lines: &[&str], start: usize, ctx: &mut ScriptContext) -> usize {
    // Gather condition spanning possibly multiple lines up to "then"
    let first = lines[start].trim();
    let cond_start = first.strip_prefix("if ").unwrap_or("");
    let mut cond_parts = vec![cond_start.to_string()];
    let mut i = start + 1;
    loop {
        if i >= lines.len() { break; }
        let line = lines[i].trim();
        if line == "then" { i += 1; break; }
        cond_parts.push(line.to_string());
        i += 1;
    }

    let cond = cond_parts.join(" ");
    let expanded_cond = expand_vars(&cond, ctx);
    let cond_tokens = crate::parser::tokenize(&expanded_cond).unwrap_or_default();
    let cond_pipelines = crate::parser::parse(&cond_tokens);
    let cond_exit = crate::executor::execute_pipelines(cond_pipelines);

    let condition_met = cond_exit == 0;
    let mut branch_taken = false;

    if condition_met {
        branch_taken = true;
        while i < lines.len() {
            let trimmed = lines[i].trim();
            if trimmed.starts_with("elif ") || trimmed.starts_with("else") || trimmed.starts_with("fi") { break; }
            if trimmed.starts_with("if ") { i = execute_if(lines, i, ctx); continue; }
            if trimmed.starts_with("for ") { let (ni,_) = execute_for(lines, i, ctx); i = ni; continue; }
            if trimmed.starts_with("while ") || trimmed.starts_with("until ") { let (ni,_) = execute_while(lines, i, ctx); i = ni; continue; }
            execute_line_trimmed(&trimmed, ctx);
            i += 1;
        }
    }

    if !branch_taken && i < lines.len() {
        let line_at_i = lines[i].trim();
        if line_at_i.starts_with("elif ") || line_at_i.starts_with("else") {
            if line_at_i.starts_with("elif ") {
                return execute_if_elif(lines, i, ctx);
            }
            // else branch
            i += 1;
            while i < lines.len() {
                let trimmed = lines[i].trim();
                if trimmed.starts_with("fi") { break; }
                if trimmed.starts_with("if ") { i = execute_if(lines, i, ctx); continue; }
                if trimmed.starts_with("for ") { let (ni,_) = execute_for(lines, i, ctx); i = ni; continue; }
                if trimmed.starts_with("while ") || trimmed.starts_with("until ") { let (ni,_) = execute_while(lines, i, ctx); i = ni; continue; }
                execute_line_trimmed(&trimmed, ctx);
                i += 1;
            }
        }
    }

    while i < lines.len() && !lines[i].trim().starts_with("fi") { i += 1; }
    if i < lines.len() { i + 1 } else { i }
}

fn execute_if_elif(lines: &[&str], start: usize, ctx: &mut ScriptContext) -> usize {
    let first = lines[start].trim();
    let cond_start = first.strip_prefix("elif ").unwrap_or("");
    let mut cond_parts = vec![cond_start.to_string()];
    let mut i = start + 1;
    loop {
        if i >= lines.len() { break; }
        let line = lines[i].trim();
        if line == "then" { i += 1; break; }
        cond_parts.push(line.to_string());
        i += 1;
    }

    let cond = cond_parts.join(" ");
    let expanded_cond = expand_vars(&cond, ctx);
    let cond_tokens = crate::parser::tokenize(&expanded_cond).unwrap_or_default();
    let cond_pipelines = crate::parser::parse(&cond_tokens);
    let cond_exit = crate::executor::execute_pipelines(cond_pipelines);

    let condition_met = cond_exit == 0;
    let mut branch_taken = false;

    if condition_met {
        branch_taken = true;
        while i < lines.len() {
            let trimmed = lines[i].trim();
            if trimmed.starts_with("elif ") || trimmed.starts_with("else") || trimmed.starts_with("fi") { break; }
            if trimmed.starts_with("if ") { i = execute_if(lines, i, ctx); continue; }
            if trimmed.starts_with("for ") { let (ni,_) = execute_for(lines, i, ctx); i = ni; continue; }
            if trimmed.starts_with("while ") || trimmed.starts_with("until ") { let (ni,_) = execute_while(lines, i, ctx); i = ni; continue; }
            execute_line_trimmed(&trimmed, ctx);
            i += 1;
        }
    }

    if !branch_taken && i < lines.len() {
        let line_at_i = lines[i].trim();
        if line_at_i.starts_with("elif ") {
            return execute_if_elif(lines, i, ctx);
        }
        if line_at_i.starts_with("else") {
            i += 1;
            while i < lines.len() {
                let trimmed = lines[i].trim();
                if trimmed.starts_with("fi") { break; }
                if trimmed.starts_with("if ") { i = execute_if(lines, i, ctx); continue; }
                if trimmed.starts_with("for ") { let (ni,_) = execute_for(lines, i, ctx); i = ni; continue; }
                if trimmed.starts_with("while ") || trimmed.starts_with("until ") { let (ni,_) = execute_while(lines, i, ctx); i = ni; continue; }
                execute_line_trimmed(&trimmed, ctx);
                i += 1;
            }
        }
    }

    while i < lines.len() && !lines[i].trim().starts_with("fi") { i += 1; }
    if i < lines.len() { i + 1 } else { i }
}

fn execute_for(lines: &[&str], start: usize, ctx: &mut ScriptContext) -> (usize, bool) {
    let first = lines[start].trim();
    let rest = first.strip_prefix("for ").unwrap_or("");
    let parts: Vec<&str> = rest.splitn(3, ' ').collect();
    if parts.len() < 3 || parts[1] != "in" { return (start + 1, false); }
    let var_name = parts[0];
    let word_str = parts[2].strip_suffix(';').unwrap_or(parts[2]);
    let words: Vec<&str> = word_str.split(' ').filter(|s| !s.is_empty()).collect();

    let mut i = start + 1;
    while i < lines.len() && lines[i].trim() != "do" { i += 1; }
    i += 1;

    let mut body_lines: Vec<&str> = Vec::new();
    let mut depth = 1;
    while i < lines.len() && depth > 0 {
        let line = lines[i].trim();
        if (line.starts_with("for ") || line.starts_with("while ") || line.starts_with("until ") || line.starts_with("if ")) && !line.starts_with("fi") { depth += 1; }
        if line == "done" { depth -= 1; if depth == 0 { break; } }
        body_lines.push(lines[i]);
        i += 1;
    }

    ctx.break_flag = false;
    ctx.continue_flag = false;
    for word in &words {
        if ctx.break_flag { break; }
        ctx.vars.push((var_name.to_string(), word.to_string()));
        let content = body_lines.join("\n");
        if ctx.continue_flag { ctx.continue_flag = false; }
        if !ctx.break_flag {
            execute_lines(&content, ctx);
        }
        ctx.vars.pop();
    }
    ctx.break_flag = false;
    ctx.continue_flag = false;

    (i + 1, true)
}

fn execute_while(lines: &[&str], start: usize, ctx: &mut ScriptContext) -> (usize, bool) {
    let first = lines[start].trim();
    let is_until = first.starts_with("until ");
    let rest = if is_until { first.strip_prefix("until ").unwrap_or("") } else { first.strip_prefix("while ").unwrap_or("") };

    let mut cond_parts = vec![rest.to_string()];
    let mut i = start + 1;
    while i < lines.len() && lines[i].trim() != "do" {
        cond_parts.push(lines[i].trim().to_string());
        i += 1;
    }
    i += 1;

    let mut body_lines: Vec<&str> = Vec::new();
    let mut depth = 1;
    while i < lines.len() && depth > 0 {
        let line = lines[i].trim();
        if (line.starts_with("for ") || line.starts_with("while ") || line.starts_with("until ") || line.starts_with("if ")) && !line.starts_with("fi") { depth += 1; }
        if line == "done" { depth -= 1; if depth == 0 { break; } }
        body_lines.push(lines[i]);
        i += 1;
    }

    ctx.break_flag = false;
    ctx.continue_flag = false;
    loop {
        if ctx.break_flag { break; }
        let cond = cond_parts.join(" ");
        let expanded = expand_vars(&cond, ctx);
        let tokens = crate::parser::tokenize(&expanded).unwrap_or_default();
        let pip = crate::parser::parse(&tokens);
        let cond_exit = crate::executor::execute_pipelines(pip);

        let met = cond_exit == 0;
        if is_until { if met { break; } } else { if !met { break; } }

        if ctx.continue_flag { ctx.continue_flag = false; }
        let content = body_lines.join("\n");
        execute_lines(&content, ctx);
    }
    ctx.break_flag = false;
    ctx.continue_flag = false;

    (i + 1, true)
}

fn execute_case(lines: &[&str], start: usize, ctx: &mut ScriptContext) -> usize {
    let first = lines[start].trim();
    let rest = first.strip_prefix("case ").unwrap_or("");
    let word = rest.split_whitespace().next().unwrap_or("").to_string();
    let mut i = start + 1;
    let mut matched = false;

    while i < lines.len() {
        let line = lines[i].trim();
        if line.starts_with("esac") { i += 1; break; }
        if line.ends_with(')') {
            let pattern = line.trim_end_matches(')').trim();
            let expanded_word = expand_vars(&word, ctx);
            let matches = pattern.split('|').any(|p| glob_match(p.trim(), &expanded_word));
            if matches { matched = true; }
            i += 1;
            while i < lines.len() {
                let l = lines[i].trim();
                if l == ";;" { i += 1; break; }
                if l.starts_with("esac") { break; }
                if matched { execute_line_trimmed(&l, ctx); }
                i += 1;
            }
            if matched { break; }
        } else {
            i += 1;
        }
    }
    while i < lines.len() && !lines[i].trim().starts_with("esac") { i += 1; }
    i + 1
}

fn define_function(lines: &[&str], start: usize, ctx: &mut ScriptContext) -> (usize, bool) {
    let first = lines[start].trim();
    let name = if let Some(rest) = first.strip_prefix("function ") {
        rest.split_whitespace().next().unwrap_or("")
    } else {
        first.split_whitespace().next().unwrap_or("")
    };
    let mut i = start + 1;
    while i < lines.len() && lines[i].trim() != "{" { i += 1; }
    i += 1;
    let mut body: Vec<String> = Vec::new();
    let mut depth = 1;
    while i < lines.len() && depth > 0 {
        let trimmed = lines[i].trim();
        if trimmed == "{" { depth += 1; }
        if trimmed == "}" { depth -= 1; if depth == 0 { break; } }
        body.push(lines[i].to_string());
        i += 1;
    }
    ctx.funcs.push((name.to_string(), body.clone()));
    crate::define_function(name, &body);
    (i + 1, true)
}

fn expand_vars(input: &str, ctx: &ScriptContext) -> String {
    let mut out = String::new();
    let chars: Vec<char> = input.chars().collect();
    let mut pos = 0;
    while pos < chars.len() {
        if chars[pos] == '$' && pos + 1 < chars.len() {
            match chars[pos + 1] {
                '{' => {
                    if let Some(end) = input[pos+2..].find('}') {
                        let var = &input[pos+2..pos+2+end];
                        out.push_str(&lookup_var(var, ctx));
                        pos += 3 + end;
                    } else { out.push(chars[pos]); pos += 1; }
                }
                c if c.is_digit(10) => {
                    let idx = (c as u8 - b'0') as usize;
                    let val = if idx == 0 { ctx.pos_args.first().cloned().unwrap_or_default() }
                              else { ctx.pos_args.get(idx - 1).cloned().unwrap_or_default() };
                    out.push_str(&val);
                    pos += 2;
                }
                '@' => {
                    out.push_str(&ctx.pos_args.join(" "));
                    pos += 2;
                }
                '*' => {
                    out.push_str(&ctx.pos_args.join(" "));
                    pos += 2;
                }
                '#' => {
                    out.push_str(&alloc::format!("{}", ctx.pos_args.len()));
                    pos += 2;
                }
                '?' => { out.push_str(&alloc::format!("{}", crate::get_last_exit())); pos += 2; }
                '$' => { pos += 2; }
                c if c.is_alphabetic() || c == '_' => {
                    let s = pos + 1;
                    let mut e = s;
                    while e < chars.len() && (chars[e].is_alphanumeric() || chars[e] == '_') { e += 1; }
                    out.push_str(&lookup_var(&input[s..e], ctx));
                    pos = e;
                }
                _ => { out.push(chars[pos]); pos += 1; }
            }
        } else {
            out.push(chars[pos]);
            pos += 1;
        }
    }
    out
}

fn lookup_var(name: &str, ctx: &ScriptContext) -> String {
    for (k, v) in &ctx.vars { if k == name { return v.clone(); } }
    crate::get_env(name).unwrap_or_default()
}

fn glob_match(pattern: &str, word: &str) -> bool {
    let pat: Vec<char> = pattern.chars().collect();
    let w: Vec<char> = word.chars().collect();
    glob_inner(&pat, &w, 0, 0)
}

fn glob_inner(p: &[char], w: &[char], pi: usize, wi: usize) -> bool {
    if pi >= p.len() { return wi >= w.len(); }
    match p[pi] {
        '*' => {
            if pi + 1 >= p.len() { return true; }
            let mut j = wi;
            while j <= w.len() {
                if glob_inner(p, w, pi + 1, j) { return true; }
                j += 1;
            }
            false
        }
        '?' => wi < w.len() && glob_inner(p, w, pi + 1, wi + 1),
        c => wi < w.len() && w[wi] == c && glob_inner(p, w, pi + 1, wi + 1),
    }
}

fn read_file(path: &str) -> String {
    let c_str = match CString::new(path.as_bytes()) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };
    let fd = unsafe { libsarga::syscall::syscall2(2, c_str.as_ptr() as u64, 0u64) };
    if fd < 0 { return String::new(); }
    let mut buf = [0u8; 8192];
    let mut out = String::new();
    loop {
        let n = libsarga::io::read(fd, &mut buf).unwrap_or(0);
        if n == 0 { break; }
        out.push_str(core::str::from_utf8(&buf[..n]).unwrap_or(""));
    }
    let _ = unsafe { libsarga::syscall::syscall1(3, fd as u64) };
    out
}
