use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub enum CsvError {
    Io(std::io::Error),
    Parse(String),
}

impl fmt::Display for CsvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CsvError::Io(e) => write!(f, "IO error: {e}"),
            CsvError::Parse(msg) => write!(f, "Parse error: {msg}"),
        }
    }
}

impl From<std::io::Error> for CsvError {
    fn from(e: std::io::Error) -> Self {
        CsvError::Io(e)
    }
}

#[derive(Debug, Clone)]
pub struct CsvRecord {
    pub fields: Vec<String>,
}

impl CsvRecord {
    pub fn get(&self, index: usize) -> Option<&str> {
        self.fields.get(index).map(String::as_str)
    }
}

#[derive(Debug)]
pub struct CsvFile {
    pub headers: Option<CsvRecord>,
    pub records: Vec<CsvRecord>,
}

impl CsvFile {
    pub fn column(&self, name: &str) -> Option<Vec<&str>> {
        let headers = self.headers.as_ref()?;
        let idx = headers.fields.iter().position(|h| h == name)?;
        Some(self.records.iter().filter_map(|r| r.get(idx)).collect())
    }
}

pub struct CsvParser {
    delimiter: char,
    has_headers: bool,
}

impl CsvParser {
    pub fn new() -> Self {
        CsvParser { delimiter: ',', has_headers: true }
    }

    pub fn delimiter(mut self, d: char) -> Self {
        self.delimiter = d;
        self
    }

    pub fn has_headers(mut self, v: bool) -> Self {
        self.has_headers = v;
        self
    }

    pub fn parse_str(&self, input: &str) -> Result<CsvFile, CsvError> {
        let mut records: Vec<CsvRecord> = Vec::new();

        for (line_num, line) in input.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let fields = self
                .parse_line(line)
                .map_err(|e| CsvError::Parse(format!("line {}: {}", line_num + 1, e)))?;
            records.push(CsvRecord { fields });
        }

        let (headers, records) = if self.has_headers && !records.is_empty() {
            let h = records.remove(0);
            (Some(h), records)
        } else {
            (None, records)
        };

        Ok(CsvFile { headers, records })
    }

    pub fn parse_file(&self, path: impl AsRef<Path>) -> Result<CsvFile, CsvError> {
        let content = fs::read_to_string(path)?;
        self.parse_str(&content)
    }

    fn parse_line(&self, line: &str) -> Result<Vec<String>, String> {
        let mut fields = Vec::new();
        let mut chars = line.chars().peekable();

        loop {
            let field = if chars.peek() == Some(&'"') {
                chars.next(); // consume opening quote
                self.parse_quoted(&mut chars)?
            } else {
                self.parse_unquoted(&mut chars)
            };

            fields.push(field);

            match chars.next() {
                Some(c) if c == self.delimiter => {}
                None => break,
                Some(c) => return Err(format!("unexpected character '{c}' after field")),
            }
        }

        Ok(fields)
    }

    fn parse_quoted(
        &self,
        chars: &mut std::iter::Peekable<std::str::Chars>,
    ) -> Result<String, String> {
        let mut result = String::new();
        loop {
            match chars.next() {
                Some('"') => {
                    // escaped quote "" or closing quote
                    if chars.peek() == Some(&'"') {
                        chars.next();
                        result.push('"');
                    } else {
                        break;
                    }
                }
                Some(c) => result.push(c),
                None => return Err("unterminated quoted field".to_string()),
            }
        }
        Ok(result)
    }

    fn parse_unquoted(&self, chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
        let mut result = String::new();
        while let Some(&c) = chars.peek() {
            if c == self.delimiter {
                break;
            }
            result.push(c);
            chars.next();
        }
        result
    }
}

fn print_csv(file: &CsvFile) {
    if let Some(headers) = &file.headers {
        println!("Headers: {}", headers.fields.join(" | "));
        println!("{}", "-".repeat(40));
    }
    for (i, record) in file.records.iter().enumerate() {
        println!("[{i}] {}", record.fields.join(" | "));
    }
    println!();
}

fn main() {
    let parser = CsvParser::new();

    println!("== employees.csv ==");
    match parser.parse_file("employees.csv") {
        Ok(csv) => {
            print_csv(&csv);
            if let Some(names) = csv.column("name") {
                println!("All names: {:?}", names);
            }
            if let Some(salaries) = csv.column("salary") {
                let total: f64 = salaries.iter().filter_map(|s| s.parse::<f64>().ok()).sum();
                println!("Total salary: {total}");
            }
        }
        Err(e) => eprintln!("Error: {e}"),
    }

    println!();

    println!("== products.csv ==");
    let parser_semi = CsvParser::new().delimiter(';');
    match parser_semi.parse_file("products.csv") {
        Ok(csv) => {
            print_csv(&csv);
            if let Some(prices) = csv.column("price") {
                let max = prices
                    .iter()
                    .filter_map(|s| s.parse::<f64>().ok())
                    .fold(f64::NEG_INFINITY, f64::max);
                println!("Max price: {max}");
            }
        }
        Err(e) => eprintln!("Error: {e}"),
    }

    println!();

    println!("=== inline quoted test ===");
    let data = r#"id,title,description
1,Rust Book,"A book about ""the"" Rust language"
2,CSV Guide,"Comma, separated, values"
3,Hello World,Simple program"#;

    match parser.parse_str(data) {
        Ok(csv) => print_csv(&csv),
        Err(e) => eprintln!("Error: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parser() -> CsvParser {
        CsvParser::new()
    }

    #[test]
    fn basic_parse() {
        let csv = parser().parse_str("a,b,c\n1,2,3\n4,5,6").unwrap();
        assert_eq!(csv.headers.unwrap().fields, vec!["a", "b", "c"]);
        assert_eq!(csv.records[0].fields, vec!["1", "2", "3"]);
        assert_eq!(csv.records[1].fields, vec!["4", "5", "6"]);
    }

    #[test]
    fn quoted_field() {
        let csv = parser().parse_str("name,note\nAlice,\"hello, world\"").unwrap();
        assert_eq!(csv.records[0].fields[1], "hello, world");
    }

    #[test]
    fn escaped_quote() {
        let csv = parser().parse_str("a,b\n1,\"say \"\"hi\"\"\"").unwrap();
        assert_eq!(csv.records[0].fields[1], "say \"hi\"");
    }

    #[test]
    fn no_headers() {
        let csv = parser().has_headers(false).parse_str("1,2\n3,4").unwrap();
        assert!(csv.headers.is_none());
        assert_eq!(csv.records.len(), 2);
    }

    #[test]
    fn custom_delimiter() {
        let csv = CsvParser::new()
            .delimiter(';')
            .parse_str("a;b\n1;2")
            .unwrap();
        assert_eq!(csv.records[0].fields, vec!["1", "2"]);
    }

    #[test]
    fn column_lookup() {
        let csv = parser().parse_str("x,y\n10,20\n30,40").unwrap();
        assert_eq!(csv.column("x"), Some(vec!["10", "30"]));
        assert_eq!(csv.column("z"), None);
    }

    #[test]
    fn unterminated_quote_is_error() {
        assert!(parser().parse_str("a,b\n1,\"unclosed").is_err());
    }
}
