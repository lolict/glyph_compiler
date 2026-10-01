// 田字格坐标偏置记号 · 执行层（容器进制）
// 把记号 +1, -1, ±1, ±2, 1±.±2, ±1.-3 解析为容器步进偏置
// 偏置值就是 ±1（一个单元步），不含乘法跨步

/// 偏置类型枚举（容器步进，值恒为 ±1）
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Bias {
    Fine(i32),     // 格内微调
    Grid(i32),     // 跨声调行
    Row(i32),      // 跨声母容器
    Fam(i32),      // 跨族对角
}

/// 从记号解出所有偏置步骤（量级 = 容器层级，值只取 ±1）
/// 语法：
///   "+1"      → Fine(+1)
///   "-1"      → Fine(-1)
///   "±2"      → Grid(+1)
///   "±3"      → Row(+1)
///   "±4"      → Fam(+1)
///   "±2.-3"   → [Grid(+1), Row(-1)]
pub fn decode(s: &str) -> Vec<Bias> {
    let mut out = Vec::new();
    for tok in s.split('.') {
        let tok = tok.trim();
        if tok.is_empty() { continue; }

        let has_neg = tok.contains('-');
        let has_pos = tok.contains('+');
        let sign: i32 = if has_neg && !has_pos { -1 } else { 1 };

        let num_str: String = tok.chars()
            .filter(|c| c.is_ascii_digit())
            .collect();
        let mag: i32 = num_str.parse().unwrap_or(1);

        out.push(match mag {
            1 => Bias::Fine(sign),
            2 => Bias::Grid(sign),
            3 => Bias::Row(sign),
            4 => Bias::Fam(sign),
            _ => Bias::Fine(sign),
        });
    }
    out
}

/// 偏置汇总为三个维度的步进 (ds, dt, dy)
/// 注意：地址不再是 base 数，而是三元组 (s, t, y)
/// 这里返回的是声/韵/调三个维度的 ±1 步进
pub fn apply_xyz(biases: &[Bias]) -> (i32, i32, i32) {
    let mut ds = 0i32;
    let mut dt = 0i32;
    let mut dy = 0i32;
    for b in biases {
        match b {
            Bias::Fine(d) => { dy += *d; }   // Fine 在韵母维微调
            Bias::Grid(d) => { dt += *d; }    // Grid 跨声调行
            Bias::Row(d) => { ds += *d; }     // Row 跨声母容器
            Bias::Fam(d) => { ds += *d; dt += *d; }  // Fam 对角跨
        }
    }
    (ds, dt, dy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        assert_eq!(decode("+1"), vec![Bias::Fine(1)]);
        assert_eq!(decode("-1"), vec![Bias::Fine(-1)]);
        assert_eq!(decode("±1"), vec![Bias::Fine(1)]);
        assert_eq!(decode("±2"), vec![Bias::Grid(1)]);
        assert_eq!(decode("±3"), vec![Bias::Row(1)]);
        assert_eq!(decode("±4"), vec![Bias::Fam(1)]);
    }

    #[test]
    fn test_compound() {
        assert_eq!(decode("±2.-3"), vec![Bias::Grid(1), Bias::Row(-1)]);
        assert_eq!(decode("-1.2+"), vec![Bias::Fine(-1), Bias::Grid(1)]);
    }

    #[test]
    fn test_apply_xyz() {
        // +1 → 韵母维微进 1
        assert_eq!(apply_xyz(&decode("+1")), (0, 0, 1));
        // -2 → 声调维退 1
        assert_eq!(apply_xyz(&decode("-2")), (0, -1, 0));
        // ±3 → 声母维进 1
        assert_eq!(decode("±3").iter().map(|b| apply_xyz(&[*b]).0).next().unwrap(), 1);
    }
}
