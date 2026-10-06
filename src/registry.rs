// 满全法 · 住客注册清单
// 特征编程核心：名称地址 = 完整统计范畴，遍历注册表 = 数出所有地址
// 格式：地址在前，命名空间在后；四维身份 + 状态 + 履历 + 公约
//
// 容量声明（声母数 / 韵母数 / 声调数 / 系 / 列）不由本文件定义。
// 全数由 registry.toml [容量声明] 驱动，程序仅读取。
// 程序不得擅自创造进制数量。

// ═══════════════════════════════════════════════════
// 住客状态
// ═══════════════════════════════════════════════════

#[derive(Clone, Copy, PartialEq)]
pub enum ResidenceStatus {
    Vacant = 0,     // 空置
    Occupied = 1,   // 已入住
    Mirror = 2,     // 镜像 — 同一住客的另一面
    Fracture = 3,   // 断裂 — 碰撞散乱后退回
}

// ═══════════════════════════════════════════════════
// 碰撞结果
// ═══════════════════════════════════════════════════

#[derive(Clone, Copy)]
pub enum CollideResult {
    Merge = 0,      // 合一 — 合并产生新住客
    Fracture = 1,   // 断裂 — 不合并，格子空出
    Mirror = 2,     // 镜像 — 同一住客两面合一
}

// ═══════════════════════════════════════════════════
// 住客四维身份 — 颜色 / 声音 / 相位 / 数字
// ═══════════════════════════════════════════════════

pub struct ColorCol {
    series: u32,        // 色系（由容量声明的 series_count 派生）
    col: u32,           // 列位（由容量声明的 col_count 派生）
    hue: f64,           // 色相角（由色系列 + 列 派生，不设上限常量）
    idx: u32,           // 色序号（= series × col_count + col）
}

struct VoiceCol {
    shengmu_id: u32,    // 声母序号（< 容量声明的 shengmu_cap）
    yunmu_id: u32,      // 韵母序号（< 容量声明的 yunmu_cap）
    tone_id: u32,       // 声调序号（< 容量声明的 tone_cap）
    label: u8,          // 声母拉丁代表字（有限存储）
}

struct PhaseCol {
    angle: f64,         // 相位角
    mirror_angle: f64,  // 镜像对称角（径向对称群）
}

struct NumeralCol {
    value: u32,         // 纯数编号
    addr: u32,          // 容器地址（进制序号）
}

// ═══════════════════════════════════════════════════
// 碰撞履历记录
// ═══════════════════════════════════════════════════

struct CollideRecord {
    opponent: u32,          // 对手住客 id
    result: CollideResult,  // 碰撞结果
    addr_before: u32,       // 碰撞前地址
    _pad: u8,               // 对齐
}

// ═══════════════════════════════════════════════════
// 住客条目
// ═══════════════════════════════════════════════════

pub struct Resident {
    id: u32,                        // 序号（自然增长）
    addr: u32,                      // 地址（进制序号）
    ns: u8,                         // 命名空间类型（枚举编码）
    color: ColorCol,                // 颜色列
    voice: VoiceCol,                // 声音列
    phase: PhaseCol,                // 相位列
    numeral: NumeralCol,            // 数字列
    status: ResidenceStatus,        // 入住状态
    version: u32,                   // 版本号（碰撞后递增）
    history_count: u32,             // 当前履历条数
    history: [CollideRecord; 16],   // 履历数组（16 = 缓冲上限，非住客进制）
    convention_id: u32,             // 引用公约 id（0xFF = 无）
    notes: u32,                     // 约束方案引用（地址指针）
    next_expect: u32,               // 预期下一次碰撞对手 id
}

// ═══════════════════════════════════════════════════
// 公约条目
// ═══════════════════════════════════════════════════

pub struct Convention {
    id: u32,                        // 公约序号
    baseline: u32,                  // 公约基准值（固定）
    queue_count: u32,               // 排队者数量
    queue: [u32; 16],               // 排队者 id 列表（16 = 缓冲上限）
    ns: u8,                         // 所属命名空间
    trigger_count: u32,             // 触发计数（≥3 时生成）
}

// ═══════════════════════════════════════════════════
// 容量声明 — 由 registry.toml 载入，程序不自创
// ═══════════════════════════════════════════════════

pub struct Capacity {
    pub shengmu_cap: u32,       // 声母基数（如 24）
    pub yunmu_cap: u32,         // 韵母基数（如 29-33，由你定）
    pub tone_cap: u32,          // 声调进制（如 60）
    pub series_count: u32,      // 系（如 6）
    pub col_count: u32,         // 每系列列数（如 7）
    pub color_count: u32,       // 颜色总数 = series × col
    pub max_residents: usize,   // 住客上限
    pub max_conventions: usize, // 公约条目上限
}

impl Capacity {
    // 默认值仅用于 toml 文件缺失字段时的兜底
    // 优先从 registry.toml [容量声明] 读
    fn default() -> Capacity {
        Capacity {
            shengmu_cap: 24,
            yunmu_cap: 30,       // 韵母基数暂定 30，以 toml 为准
            tone_cap: 60,
            series_count: 6,
            col_count: 7,
            color_count: 42,     // series × col = 6 × 7
            max_residents: 256,
            max_conventions: 32,
        }
    }

    // 极简 TOML 解析：只读 [容量声明] 段的 key = value 行
    // 不引入 toml crate，纯字符串匹配
    pub fn from_toml(path: &str) -> Capacity {
        let mut cap = Capacity::default();
        let data = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => return cap,  // 文件不存在 → 用默认值
        };
        let lines: Vec<&str> = data.lines().collect();
        let mut in_section = false;
        for line in lines {
            let trimmed = line.trim();
            if trimmed == "[容量声明]" {
                in_section = true;
                continue;
            }
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                in_section = false;
                continue;
            }
            if !in_section { continue; }
            // 跳过空行和注释
            if trimmed.is_empty() || trimmed.starts_with('#') { continue; }
            // key = value
            if let Some(eq) = trimmed.find('=') {
                let key = trimmed[..eq].trim();
                let val = trimmed[eq+1..].trim();
                // 去掉行内注释
                let val = if let Some(p) = val.find('#') { val[..p].trim() } else { val };
                let num: u32 = val.parse().unwrap_or(0);
                if num == 0 { continue; }
                match key {
                    "声母基数" => cap.shengmu_cap = num,
                    "韵母基数" => cap.yunmu_cap = num,
                    "声调进制" => cap.tone_cap = num,
                    "系" => cap.series_count = num,
                    "每系列列数" => cap.col_count = num,
                    "住客上限" => cap.max_residents = num as usize,
                    "公约上限" => cap.max_conventions = num as usize,
                    _ => {}
                }
            }
        }
        cap.color_count = cap.series_count * cap.col_count;
        cap
    }
}

// ═══════════════════════════════════════════════════
// 韵母符号表 — 从 registry.toml [韵母符号表] 加载
// 小写 = 声调符号池，大写 = 韵母符号池（大小写天然区分层）
// ═══════════════════════════════════════════════════

pub struct YunmuTable {
    // index → Unicode符号字符串
    pub symbols: Vec<String>,
    // Unicode符号字符串 → index（反向查找）
    lookup: std::collections::HashMap<String, u32>,
}

impl YunmuTable {
    // 空表（默认）
    fn empty() -> YunmuTable {
        YunmuTable {
            symbols: Vec::new(),
            lookup: std::collections::HashMap::new(),
        }
    }

    // 从 registry.toml 的 [韵母符号表.小写] + [韵母符号表.大写] 段加载
    // 不引入 toml crate，纯字符串逐行解析
    // 节标题可能带注释：允 [韵母符号表.小写] # 注释 这种形式
    pub fn from_toml(path: &str) -> YunmuTable {
        let mut table = YunmuTable::empty();
        let data = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => return table,
        };
        let mut in_yunmu_section = false;
        for line in data.lines() {
            let t = line.trim();
            // 节标题匹配：去掉右侧注释后比较
            let section_name = t.split('#').next().unwrap_or("").trim();
            if section_name == "[韵母符号表.小写]" || section_name == "[韵母符号表.大写]" {
                in_yunmu_section = true;
                continue;
            }
            // 其他节标题 → 退出
            if section_name.starts_with('[') && section_name.ends_with(']') {
                in_yunmu_section = false;
                continue;
            }
            if !in_yunmu_section { continue; }
            if t.is_empty() || t.starts_with('#') { continue; }
            // 解析 "ȧ = { ..."：取 = 左侧为符号
            if let Some(eq_pos) = t.find('=') {
                let symbol = t[..eq_pos].trim();
                if symbol.is_empty() { continue; }
                let idx = table.symbols.len() as u32;
                table.symbols.push(symbol.to_string());
                table.lookup.insert(symbol.to_string(), idx);
            }
        }
        table
    }

    // 符号 → index，找不到返回 0xFFFFFFFF
    pub fn symbol_to_index(&self, symbol: &str) -> u32 {
        match self.lookup.get(symbol) {
            Some(&idx) => idx,
            None => 0xFFFFFFFF,
        }
    }

    // index → 符号，越界返回空串
    pub fn index_to_symbol(&self, idx: u32) -> &str {
        if (idx as usize) < self.symbols.len() {
            &self.symbols[idx as usize]
        } else {
            ""
        }
    }

    // 符号总数 = 进制池容量
    pub fn len(&self) -> u32 {
        self.symbols.len() as u32
    }

    // 判断符号归属：大写开头 = 韵母池，小写开头 = 声调池
    pub fn owner(&self, symbol: &str) -> &str {
        if symbol.is_empty() { return ""; }
        // 取首字符判断大小写
        let first = symbol.chars().next().unwrap();
        if first.is_uppercase() { "韵母" } else { "声调" }
    }
}

// ═══════════════════════════════════════════════════
// 注册清单
// ═══════════════════════════════════════════════════

pub struct Registry {
    residents: Vec<Resident>,       // 住客表（Vec 不硬编码容量）
    conventions: Vec<Convention>,   // 公约表
    capacity: Capacity,             // 容量声明（来自 toml）
    yunmu_table: YunmuTable,        // 韵母符号表（来自 toml）
    count: u32,                     // 住客计数
    convention_count: u32,          // 公约计数
    next_id: u32,                   // 下一可用住客 id
}

// ═══════════════════════════════════════════════════
// 命名空间类型枚举（编码 → 查表得字符串）
// ═══════════════════════════════════════════════════

// 0 = 声母, 1 = 韵母, 2 = 声调
// 3 = 天干, 4 = 地支, 5 = 八卦
// 6 = 化学元素, 7 = 星座, 8 = 数字进制
// 9+ = 预留扩展

// ═══════════════════════════════════════════════════
// 四维计算 — 住客间距离
// ═══════════════════════════════════════════════════

// 颜色距离 = |a - b|，环状（color_count 闭环）
fn color_distance(a: u32, b: u32, color_count: u32) -> u32 {
    let d = if a > b { a - b } else { b - a };
    // 闭环：从另一边绕更短取
    let half = color_count / 2;
    if d > half { color_count - d } else { d }
}

// 相位角度差（0-180 之间最小弧）
fn phase_distance(a: f64, b: f64) -> f64 {
    let d = (a - b).abs();
    if d > 180.0 { 360.0 - d } else { d }
}

// 声音距离：声母差 + 韵母差 + 声调差，三维叠加
fn voice_distance(a: &VoiceCol, b: &VoiceCol) -> u32 {
    let sd = if a.shengmu_id > b.shengmu_id {
        a.shengmu_id - b.shengmu_id
    } else {
        b.shengmu_id - a.shengmu_id
    };
    let yd = if a.yunmu_id > b.yunmu_id {
        a.yunmu_id - b.yunmu_id
    } else {
        b.yunmu_id - a.yunmu_id
    };
    let td = if a.tone_id > b.tone_id {
        a.tone_id - b.tone_id
    } else {
        b.tone_id - a.tone_id
    };
    sd + yd + td
}

// 数字步阶差 = |a - b|
fn numeral_distance(a: u32, b: u32) -> u32 {
    if a > b { a - b } else { b - a }
}

// ═══════════════════════════════════════════════════
// 色相角计算（由容量声明的 series_count / col_count 派生）
// hue_angle = series × (360 / series_count) + col × (360 / series_count / col_count)
// ═══════════════════════════════════════════════════

fn hue_angle(series: u32, col: u32, cap: &Capacity) -> f64 {
    let s = series as f64;
    let c = col as f64;
    let sc = cap.series_count as f64;
    let cc = cap.col_count as f64;
    s * (360.0 / sc) + c * (360.0 / sc / cc)
}

// 镜像对称角：关于 180° 中心对称
fn mirror_hue(hue: f64) -> f64 {
    (360.0 - hue) % 360.0
}

// ═══════════════════════════════════════════════════
// 住客注册
// ═══════════════════════════════════════════════════

impl Registry {
    // ─── 空创建 ───
    fn new(cap: Capacity) -> Registry {
        Registry {
            residents: Vec::with_capacity(cap.max_residents),
            conventions: Vec::with_capacity(cap.max_conventions),
            capacity: cap,
            yunmu_table: YunmuTable::empty(),
            count: 0,
            convention_count: 0,
            next_id: 0,
        }
    }

    // ─── 从 TOML 容量声明 + 韵母符号表创建 ───
    fn from_toml_file(path: &str) -> Registry {
        let cap = Capacity::from_toml(path);
        let mut reg = Registry::new(cap);
        reg.yunmu_table = YunmuTable::from_toml(path);
        reg
    }

    // ─── 通过符号入住（符号 → index → 分配）───
    fn register_by_symbol(&mut self, addr: u32, ns: u8,
                          color_idx: u32,
                          shengmu_id: u32, yunmu_symbol: &str, tone_id: u32) -> u32 {
        let yunmu_id = self.yunmu_table.symbol_to_index(yunmu_symbol);
        if yunmu_id == 0xFFFFFFFF { return 0xFFFFFFFF; }
        self.register(addr, ns, color_idx, shengmu_id, yunmu_id, tone_id)
    }

    // ─── 入住 ───
    fn register(&mut self, addr: u32, ns: u8,
                color_idx: u32,
                shengmu_id: u32, yunmu_id: u32, tone_id: u32) -> u32 {
        if self.count as usize >= self.capacity.max_residents { return 0xFFFFFFFF; }

        let id = self.next_id;

        let series = color_idx / self.capacity.col_count;
        let col = color_idx % self.capacity.col_count;
        let hue = hue_angle(series, col, &self.capacity);

        let resident = Resident {
            id: id,
            addr: addr,
            ns: ns,
            color: ColorCol {
                series: series,
                col: col,
                hue: hue,
                idx: color_idx,
            },
            voice: VoiceCol {
                shengmu_id: shengmu_id,
                yunmu_id: yunmu_id,
                tone_id: tone_id,
                label: 0,
            },
            phase: PhaseCol {
                angle: hue,
                mirror_angle: mirror_hue(hue),
            },
            numeral: NumeralCol {
                value: id,
                addr: addr,
            },
            status: ResidenceStatus::Occupied,
            version: 1,
            history_count: 0,
            history: unsafe { std::mem::zeroed() },
            convention_id: 0xFF,
            notes: 0,
            next_expect: 0xFF,
        };

        self.residents.push(resident);
        self.count += 1;
        self.next_id += 1;
        id
    }

    // ─── 按 id 查住客 ───
    fn find(&self, id: u32) -> usize {
        for i in 0..self.count as usize {
            if self.residents[i].id == id { return i; }
        }
        0xFFFFFFFF as usize
    }

    // ─── 按地址查住客 ───
    fn find_by_addr(&self, addr: u32, ns: u8) -> usize {
        for i in 0..self.count as usize {
            if self.residents[i].addr == addr && self.residents[i].ns == ns {
                return i;
            }
        }
        0xFFFFFFFF as usize
    }

    // ─── 碰撞判定（按优先级）───
    fn collide(&mut self, id_a: u32, id_b: u32) -> (CollideResult, usize, usize) {
        let ia = self.find(id_a);
        let ib = self.find(id_b);
        if ia == 0xFFFFFFFF as usize || ib == 0xFFFFFFFF as usize {
            return (CollideResult::Fracture, ia, ib);
        }
        if ia >= self.residents.len() || ib >= self.residents.len() {
            return (CollideResult::Fracture, ia, ib);
        }

        let phase_dist = phase_distance(
            self.residents[ia].phase.angle,
            self.residents[ib].phase.angle
        );

        // 优先级一：镜像
        if (phase_dist - 180.0).abs() < 7.0 {
            return self.do_mirror(ia, ib);
        }

        // 优先级二：合并
        let cd = color_distance(
            self.residents[ia].color.idx,
            self.residents[ib].color.idx,
            self.capacity.color_count
        );
        let vd = voice_distance(&self.residents[ia].voice, &self.residents[ib].voice);
        if cd == 0 && vd <= 12 {
            return self.do_merge(ia, ib);
        }

        // 优先级三：断裂
        (CollideResult::Fracture, ia, ib)
    }

    fn do_mirror(&mut self, ia: usize, ib: usize) -> (CollideResult, usize, usize) {
        if ib < self.residents.len() {
            self.residents[ib].status = ResidenceStatus::Mirror;
        }
        let rec = CollideRecord {
            opponent: self.residents[ib].id,
            result: CollideResult::Mirror,
            addr_before: self.residents[ib].addr,
            _pad: 0,
        };
        let hc = self.residents[ia].history_count as usize;
        if hc < 16 {
            self.residents[ia].history[hc] = rec;
            self.residents[ia].history_count += 1;
        }
        self.residents[ia].version += 1;
        (CollideResult::Mirror, ia, ib)
    }

    fn do_merge(&mut self, ia: usize, ib: usize) -> (CollideResult, usize, usize) {
        if ib < self.residents.len() {
            self.residents[ib].status = ResidenceStatus::Fracture;
        }
        // 声音叠加：对容量取模以防越界（shengmu_cap / yunmu_cap 由 toml 驱动）
        self.residents[ia].voice.shengmu_id =
            (self.residents[ia].voice.shengmu_id + self.residents[ib].voice.shengmu_id)
            % self.capacity.shengmu_cap;
        self.residents[ia].voice.yunmu_id =
            (self.residents[ia].voice.yunmu_id + self.residents[ib].voice.yunmu_id)
            % self.capacity.yunmu_cap;
        self.residents[ia].version += 1;
        let rec = CollideRecord {
            opponent: self.residents[ib].id,
            result: CollideResult::Merge,
            addr_before: self.residents[ib].addr,
            _pad: 0,
        };
        let hc = self.residents[ia].history_count as usize;
        if hc < 16 {
            self.residents[ia].history[hc] = rec;
            self.residents[ia].history_count += 1;
        }
        (CollideResult::Merge, ia, ib)
    }

    // ─── 公约检测 ───
    fn check_convention(&mut self, ns: u8, color_col: u32) -> u32 {
        let mut ids: [u32; 16] = [0; 16];
        let mut cnt = 0u32;

        for i in 0..self.count as usize {
            if self.residents[i].ns == ns && self.residents[i].color.col == color_col
                && self.residents[i].status == ResidenceStatus::Occupied {
                if (cnt as usize) < 16 {
                    ids[cnt as usize] = self.residents[i].id;
                }
                cnt += 1;
            }
        }

        if cnt < 3 { return 0xFFFFFFFF; }
        if self.convention_count as usize >= self.capacity.max_conventions { return 0xFFFFFFFF; }

        let cid = self.convention_count;
        let qcnt = if cnt > 16 { 16 } else { cnt };
        let mut conv = Convention {
            id: cid,
            baseline: color_col * 10,
            queue_count: qcnt,
            queue: [0; 16],
            ns: ns,
            trigger_count: cnt,
        };
        for k in 0..conv.queue_count as usize {
            conv.queue[k] = ids[k];
            let ri = self.find(ids[k]);
            if ri < self.residents.len() {
                self.residents[ri].convention_id = cid;
            }
        }

        self.conventions.push(conv);
        self.convention_count += 1;
        cid
    }

    // ─── 遍历注册清单（特征编程核心：数出所有地址）───
    fn count_by_namespace(&self, ns: u8) -> u32 {
        let mut n = 0;
        for i in 0..self.count as usize {
            if self.residents[i].ns == ns { n += 1; }
        }
        n
    }

    fn count_by_color(&self, color_idx: u32) -> u32 {
        let mut n = 0;
        for i in 0..self.count as usize {
            if self.residents[i].color.idx == color_idx { n += 1; }
        }
        n
    }

    fn count_by_status(&self, status: ResidenceStatus) -> u32 {
        let mut n = 0;
        for i in 0..self.count as usize {
            if self.residents[i].status as u8 == status as u8 { n += 1; }
        }
        n
    }

    fn check_ascension(&self, addr: u32) -> bool {
        let mut n = 0;
        for i in 0..self.count as usize {
            if self.residents[i].addr == addr
                && self.residents[i].status == ResidenceStatus::Occupied {
                n += 1;
            }
        }
        n >= 4
    }
}

// ═══════════════════════════════════════════════════
// 测试
// ═══════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hue_angle() {
        let cap = Capacity::default();
        // 红系第0列 = 0°
        assert!((hue_angle(0, 0, &cap) - 0.0).abs() < 0.01);
        // 红系第1列 ≈ 8.57°
        assert!((hue_angle(0, 1, &cap) - 8.57).abs() < 0.1);
        // 黄系第0列 = 60°
        assert!((hue_angle(1, 0, &cap) - 60.0).abs() < 0.01);
        // 最后一列 ≈ 351.43°
        assert!((hue_angle(cap.series_count - 1, cap.col_count - 1, &cap) - 351.43).abs() < 0.1);
    }

    #[test]
    fn test_mirror_hue() {
        assert!((mirror_hue(0.0) - 0.0).abs() < 0.01);
        assert!((mirror_hue(90.0) - 270.0).abs() < 0.01);
        assert!((mirror_hue(180.0) - 180.0).abs() < 0.01);
        assert!((mirror_hue(8.57) - 351.43).abs() < 0.1);
    }

    #[test]
    fn test_color_distance() {
        let cc = 42;
        assert_eq!(color_distance(0, 1, cc), 1);
        assert_eq!(color_distance(0, 41, cc), 1); // 闭环
        assert_eq!(color_distance(0, 21, cc), 21); // 对半
        assert_eq!(color_distance(5, 10, cc), 5);
    }

    #[test]
    fn test_register() {
        let cap = Capacity::default();
        let mut reg = Registry::new(cap);
        let id0 = reg.register(0, 0, 0, 0, 0, 0);
        assert_eq!(id0, 0);
        assert_eq!(reg.count, 1);

        let id1 = reg.register(1, 0, 1, 1, 1, 1);
        assert_eq!(id1, 1);
        assert_eq!(reg.count, 2);
    }

    #[test]
    fn test_collide_mirror() {
        let cap = Capacity::default();
        let mut reg = Registry::new(cap);
        reg.register(0, 0, 0, 0, 0, 0);   // hue=0°
        reg.register(21, 0, 21, 2, 2, 2);  // hue≈180°

        let (result, _, _) = reg.collide(0, 1);
        match result {
            CollideResult::Mirror => {},
            _ => panic!("应为镜像"),
        }
    }

    #[test]
    fn test_collide_merge() {
        let cap = Capacity::default();
        let mut reg = Registry::new(cap);
        reg.register(0, 0, 0, 0, 0, 30);
        reg.register(0, 0, 0, 1, 5, 35);

        let (result, _, _) = reg.collide(0, 1);
        match result {
            CollideResult::Merge => {},
            _ => panic!("应为合一"),
        }
    }

    #[test]
    fn test_collide_fracture() {
        let cap = Capacity::default();
        let mut reg = Registry::new(cap);
        reg.register(0, 0, 0, 0, 0, 0);
        reg.register(10, 0, 10, 1, 10, 30);

        let (result, _, _) = reg.collide(0, 1);
        match result {
            CollideResult::Fracture => {},
            _ => panic!("应为断裂"),
        }
    }

    #[test]
    fn test_convention_trigger() {
        let cap = Capacity::default();
        let mut reg = Registry::new(cap);
        // 同一命名空间(ns=0)，同一 col=0 的三个住客：
        // idx=0:  col=0,  idx=7:  col=0,  idx=14: col=0
        reg.register(0, 0, 0, 0, 0, 0);
        reg.register(7, 0, 7, 1, 1, 0);
        reg.register(14, 0, 14, 2, 2, 0);

        let cid = reg.check_convention(0, 0);
        assert_ne!(cid, 0xFFFFFFFF);
        assert_eq!(reg.convention_count, 1);
    }

    #[test]
    fn test_capacity_from_toml() {
        // 写入临时 toml 文件，验证 parser 读取（使用当前进制: 48/66/70）
        let tmp = "/tmp/test_registry_capacity.toml";
        std::fs::write(tmp, r#"
# 测试用容量声明
[容量声明]
声母基数 = 48
韵母基数 = 66
声调进制 = 70
系 = 6
每系列列数 = 7
"#).unwrap();

        let cap = Capacity::from_toml(tmp);
        assert_eq!(cap.shengmu_cap, 48);
        assert_eq!(cap.yunmu_cap, 66);
        assert_eq!(cap.tone_cap, 70);
        assert_eq!(cap.series_count, 6);
        assert_eq!(cap.col_count, 7);
        assert_eq!(cap.color_count, 42); // = series × col，自动计算
    }

    #[test]
    fn test_yunmu_table_from_toml() {
        // 创建带韵母符号表的测试 toml（包含注释行，验证兼容性）
        let tmp = "/tmp/test_yunmu_table.toml";
        std::fs::write(tmp, r#"
[容量声明]
声母基数 = 48
韵母基数 = 66
声调进制 = 70

# 这是韵母符号表
[韵母符号表.小写]  # 声调池
ȧ = { base = "a", cp = "U+0227" }
ė = { base = "e", cp = "U+0117" }
ä = { base = "a", cp = "U+00E4" }

[韵母符号表.大写]  # 韵母池
Ȧ = { base = "A", cp = "U+0226", owner = "韵母" }
Ā = { base = "A", cp = "U+0100", owner = "韵母" }
"#).unwrap();

        let table = YunmuTable::from_toml(tmp);
        // 总数 = 3小写 + 2大写 = 5
        assert_eq!(table.len(), 5);

        // 小写符号 index 0-2
        assert_eq!(table.symbol_to_index("ȧ"), 0);
        assert_eq!(table.symbol_to_index("ė"), 1);
        assert_eq!(table.symbol_to_index("ä"), 2);

        // 大写符号 index 3-4
        assert_eq!(table.symbol_to_index("Ȧ"), 3);
        assert_eq!(table.symbol_to_index("Ā"), 4);

        // 不存在的符号
        assert_eq!(table.symbol_to_index("不存在的"), 0xFFFFFFFF);

        // 反向查 index → symbol
        assert_eq!(table.index_to_symbol(0), "ȧ");
        assert_eq!(table.index_to_symbol(3), "Ȧ");
        assert_eq!(table.index_to_symbol(100), ""); // 越界

        // owner 判断
        assert_eq!(table.owner("ȧ"), "声调");   // 小写 → 声调
        assert_eq!(table.owner("Ȧ"), "韵母");   // 大写 → 韵母
    }

    #[test]
    fn test_yunmu_table_full_130() {
        // 验证完整 registry.toml 加载出 130 个符号
        let toml_path = "/mnt/data/catpaw/home/workspace/硅碳心源-果套循因/toolchain/tier1/glyph_compiler/registry.toml";
        // 此测试只在文件存在时运行
        if !std::path::Path::new(toml_path).exists() {
            return;
        }
        let table = YunmuTable::from_toml(toml_path);
        // 130 = 65小写 + 65大写
        assert!(table.len() >= 129, "韵母符号表至少129个，实际 {}：请检查 toml 完整性", table.len());

        // 验证几个关键符号
        assert_ne!(table.symbol_to_index("ȧ"), 0xFFFFFFFF, "小写 ȧ 应存在");
        assert_ne!(table.symbol_to_index("Ȧ"), 0xFFFFFFFF, "大写 Ȧ 应存在");
        assert_ne!(table.symbol_to_index("ə"), 0xFFFFFFFF, "schwa ə 应存在（IPA）");
        assert_ne!(table.symbol_to_index("Ʉ"), 0xFFFFFFFF, "barred U Ʉ 应存在（IPA）");

        // 验证 owner 分类
        assert_eq!(table.owner("ȧ"), "声调");
        assert_eq!(table.owner("Ȧ"), "韵母");
    }

    #[test]
    fn test_register_by_symbol() {
        let tmp = "/tmp/test_register_symbol.toml";
        std::fs::write(tmp, r#"
[容量声明]
声母基数 = 48
韵母基数 = 66
声调进制 = 70

[韵母符号表.大写]
Ȧ = { base = "A", cp = "U+0226" }
Ā = { base = "A", cp = "U+0100" }
"#).unwrap();

        let mut reg = Registry::from_toml_file(tmp);
        // 通过大写符号 "Ȧ" 注册韵母
        let id0 = reg.register_by_symbol(5, 1, 5, 10, "Ȧ", 20);
        assert_ne!(id0, 0xFFFFFFFF);
        assert_eq!(reg.count, 1);

        // 不存在的符号 → 失败
        let id_fail = reg.register_by_symbol(6, 1, 6, 10, "不存在的", 20);
        assert_eq!(id_fail, 0xFFFFFFFF);
        assert_eq!(reg.count, 1); // 未增加
    }
}
