// 满全法 · 住客注册清单
// 特征编程核心：名称地址 = 完整统计范畴，遍历注册表 = 数出所有地址
// 格式：地址在前，命名空间在后；四维身份 + 状态 + 履历 + 公约

// ═══════════════════════════════════════════════════
// 常量
// ═══════════════════════════════════════════════════

const SERIES_COUNT: u32 = 6;       // 六系
const COL_COUNT: u32 = 7;          // 每系七列
const COLOR_COUNT: u32 = 42;       // 6×7 = 42 色
const SHENGMU_COUNT: u32 = 24;     // 24 声母
const YUNMU_COUNT: u32 = 33;       // 33 韵母（29-33 之间）
const TONE_COUNT: u32 = 60;        // 60 声调
const MAX_RESIDENTS: usize = 256;   // 住客上限
const MAX_HISTORY: usize = 16;      // 每条住客履历上限
const MAX_CONVENTIONS: usize = 32;  // 公约条目上限
const MAX_QUEUE: usize = 16;        // 公约排队列表上限

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
    series: u32,        // 色系 0-5
    col: u32,           // 列位 0-6
    hue: f64,           // 色相角 0°-360°
    idx: u32,           // 42 色序号 0-41
}

struct VoiceCol {
    shengmu_id: u32,    // 声母序号 0-23
    yunmu_id: u32,      // 韵母序号 0-32
    tone_id: u32,       // 声调序号 0-59
    label: u8,          // 声母拉丁代表字（有限存储）
}

struct PhaseCol {
    angle: f64,         // 相位角 0°-351.43°
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
    history: [CollideRecord; MAX_HISTORY],  // 履历数组
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
    queue: [u32; MAX_QUEUE],        // 排队者 id 列表
    ns: u8,                         // 所属命名空间
    trigger_count: u32,              // 触发计数（≥3 时生成）
}

// ═══════════════════════════════════════════════════
// 注册清单
// ═══════════════════════════════════════════════════

pub struct Registry {
    residents: [Resident; MAX_RESIDENTS],       // 住客表
    conventions: [Convention; MAX_CONVENTIONS], // 公约表
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

// 颜色距离 = |a - b|，环状（42 色闭环）
fn color_distance(a: u32, b: u32) -> u32 {
    let d = if a > b { a - b } else { b - a };
    // 闭环：从另一边绕更短取
    if d > 21 { 42 - d } else { d }
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
// 42 色色相角计算（从 color_quant.c 派生）
// hue_angle = layer × 60° + col × 8.57°
// ═══════════════════════════════════════════════════

fn hue_angle(series: u32, col: u32) -> f64 {
    let s = series as f64;
    let c = col as f64;
    s * (360.0 / 6.0) + c * (360.0 / 6.0 / 7.0)
}

// 镜像对称角：关于 180° 中心对称
fn mirror_hue(hue: f64) -> f64 {
    (360.0 - hue) % 360.0
}

// ═══════════════════════════════════════════════════
// 住客注册
// ═══════════════════════════════════════════════════

impl Registry {
    fn new() -> Registry {
        Registry {
            residents: unsafe {
                // 安全：Resident 全部字段都是 Copy + 有确定初值
                std::mem::zeroed()
            },
            conventions: unsafe { std::mem::zeroed() },
            count: 0,
            convention_count: 0,
            next_id: 0,
        }
    }

    // ─── 入住 ───
    fn register(&mut self, addr: u32, ns: u8,
                color_idx: u32,
                shengmu_id: u32, yunmu_id: u32, tone_id: u32) -> u32 {
        if self.count as usize >= MAX_RESIDENTS { return 0xFFFFFFFF; }

        let id = self.next_id;
        let idx = self.count as usize;

        let series = color_idx / COL_COUNT;
        let col = color_idx % COL_COUNT;
        let hue = hue_angle(series, col);

        self.residents[idx] = Resident {
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
                label: 0,  // 由调用者填写
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
    // 返回：(结果, 住客A索引, 住客B索引)
    fn collide(&mut self, id_a: u32, id_b: u32) -> (CollideResult, usize, usize) {
        let ia = self.find(id_a);
        let ib = self.find(id_b);
        if ia == 0xFFFFFFFF as usize || ib == 0xFFFFFFFF as usize {
            return (CollideResult::Fracture, ia, ib);
        }

        // 安全：两个不同索引
        let (a, b) = if ia < ib {
            let ptr = &mut self.residents as *mut [Resident; MAX_RESIDENTS];
            unsafe {
                let left = &mut (*ptr)[ia] as *mut Resident;
                let right = &mut (*ptr)[ib] as *mut Resident;
                (&mut *left, &mut *right)
            }
        } else {
            let ptr = &mut self.residents as *mut [Resident; MAX_RESIDENTS];
            unsafe {
                let left = &mut (*ptr)[ia] as *mut Resident;
                let right = &mut (*ptr)[ib] as *mut Resident;
                (&mut *left, &mut *right)
            }
        };

        // 优先级一：镜像 — 角度差接近 180°
        let phase_dist = phase_distance(a.phase.angle, b.phase.angle);
        if (phase_dist - 180.0).abs() < 7.0 {  // 允许 ±7° 容差
            return self.do_mirror(ia, ib);
        }

        // 优先级二：合并 — 颜色距离 = 0 且声音距离 ≤ 阈值
        let cd = color_distance(a.color.idx, b.color.idx);
        let vd = voice_distance(&a.voice, &b.voice);
        if cd == 0 && vd <= 12 {
            return self.do_merge(ia, ib);
        }

        // 优先级三：断裂
        (CollideResult::Fracture, ia, ib)
    }

    fn do_mirror(&mut self, ia: usize, ib: usize) -> (CollideResult, usize, usize) {
        // 镜像：B 成为 A 的一面，B 标记为 Mirror
        if ib < self.residents.len() {
            self.residents[ib].status = ResidenceStatus::Mirror;
        }
        // A 记录履历
        let rec = CollideRecord {
            opponent: self.residents[ib].id,
            result: CollideResult::Mirror,
            addr_before: self.residents[ib].addr,
            _pad: 0,
        };
        let hc = self.residents[ia].history_count as usize;
        if hc < MAX_HISTORY {
            self.residents[ia].history[hc] = rec;
            self.residents[ia].history_count += 1;
        }
        self.residents[ia].version += 1;
        (CollideResult::Mirror, ia, ib)
    }

    fn do_merge(&mut self, ia: usize, ib: usize) -> (CollideResult, usize, usize) {
        // 合并：A 吸收 B 的属性叠加，B 断裂
        if ib < self.residents.len() {
            self.residents[ib].status = ResidenceStatus::Fracture;
        }
        // A 的声音叠加取两者之和
        self.residents[ia].voice.shengmu_id =
            (self.residents[ia].voice.shengmu_id + self.residents[ib].voice.shengmu_id) % SHENGMU_COUNT;
        self.residents[ia].voice.yunmu_id =
            (self.residents[ia].voice.yunmu_id + self.residents[ib].voice.yunmu_id) % YUNMU_COUNT;
        // 版本递增
        self.residents[ia].version += 1;
        // 记录履历
        let rec = CollideRecord {
            opponent: self.residents[ib].id,
            result: CollideResult::Merge,
            addr_before: self.residents[ib].addr,
            _pad: 0,
        };
        let hc = self.residents[ia].history_count as usize;
        if hc < MAX_HISTORY {
            self.residents[ia].history[hc] = rec;
            self.residents[ia].history_count += 1;
        }
        (CollideResult::Merge, ia, ib)
    }

    // ─── 公约检测 ───
    // 同一命名空间下，同一颜色列出现 ≥3 次 → 生成公约
    fn check_convention(&mut self, ns: u8, color_col: u32) -> u32 {
        let mut ids: [u32; MAX_QUEUE] = [0; MAX_QUEUE];
        let mut cnt = 0u32;

        for i in 0..self.count as usize {
            if self.residents[i].ns == ns && self.residents[i].color.col == color_col
                && self.residents[i].status == ResidenceStatus::Occupied {
                if (cnt as usize) < MAX_QUEUE {
                    ids[cnt as usize] = self.residents[i].id;
                }
                cnt += 1;
            }
        }

        if cnt < 3 { return 0xFFFFFFFF; }

        // 生成公约
        if self.convention_count as usize >= MAX_CONVENTIONS { return 0xFFFFFFFF; }
        let cid = self.convention_count;
        let mut conv = Convention {
            id: cid,
            baseline: color_col * 10,    // 基准值 = 列号 × 10
            queue_count: if cnt > MAX_QUEUE as u32 { MAX_QUEUE as u32 } else { cnt },
            queue: [0; MAX_QUEUE],
            ns: ns,
            trigger_count: cnt,
        };
        for k in 0..conv.queue_count as usize {
            conv.queue[k] = ids[k];
            // 住客回填公约 id
            let ri = self.find(ids[k]);
            if ri < MAX_RESIDENTS {
                self.residents[ri].convention_id = cid;
            }
        }

        self.conventions[self.convention_count as usize] = conv;
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

    // 升维检查：格子住客 ≥ 4 时需要扩容
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
        // 红系第0列 = 0°
        assert!((hue_angle(0, 0) - 0.0).abs() < 0.01);
        // 红系第1列 = 8.57°
        assert!((hue_angle(0, 1) - 8.57).abs() < 0.1);
        // 黄系第0列 = 60°
        assert!((hue_angle(1, 0) - 60.0).abs() < 0.01);
        // 光系最后一列 = 351.43°
        assert!((hue_angle(5, 6) - 351.43).abs() < 0.1);
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
        assert_eq!(color_distance(0, 1), 1);
        assert_eq!(color_distance(0, 41), 1); // 闭环
        assert_eq!(color_distance(0, 21), 21); // 对半
        assert_eq!(color_distance(5, 10), 5);
    }

    #[test]
    fn test_register() {
        let mut reg = Registry::new();
        let id0 = reg.register(0, 0, 0, 0, 0, 0);
        assert_eq!(id0, 0);
        assert_eq!(reg.count, 1);

        let id1 = reg.register(1, 0, 1, 1, 1, 1);
        assert_eq!(id1, 1);
        assert_eq!(reg.count, 2);
    }

    #[test]
    fn test_collide_mirror() {
        let mut reg = Registry::new();
        // 两个住客相位差 = 180°（镜像对）
        reg.register(0, 0, 0, 0, 0, 0);   // hue=0°， idx=0
        reg.register(21, 0, 21, 2, 2, 2);  // hue=180°， idx=21

        let (result, _, _) = reg.collide(0, 1);
        match result {
            CollideResult::Mirror => {},
            _ => panic!("应为镜像"),
        }
    }

    #[test]
    fn test_collide_merge() {
        let mut reg = Registry::new();
        reg.register(0, 0, 0, 0, 0, 30);  // idx=0 红L0
        reg.register(0, 0, 0, 1, 5, 35);  // idx=0 红L0（同色不同音）

        let (result, _, _) = reg.collide(0, 1);
        match result {
            CollideResult::Merge => {},
            _ => panic!("应为合一"),
        }
    }

    #[test]
    fn test_collide_fracture() {
        let mut reg = Registry::new();
        reg.register(0, 0, 0, 0, 0, 0);   // idx=0
        reg.register(10, 0, 10, 1, 10, 30); // idx=10，声音差大

        let (result, _, _) = reg.collide(0, 1);
        match result {
            CollideResult::Fracture => {},
            _ => panic!("应为断裂"),
        }
    }

    #[test]
    fn test_convention_trigger() {
        let mut reg = Registry::new();
        // 同一命名空间(ns=0)，同一色(=0,color_col=0)，3个住客
        reg.register(0, 0, 0, 0, 0, 0);   // col=0
        reg.register(0, 0, 1, 0, 1, 0);   // col=0  (series=0,col=1)
        // 注意：需要同一 col 列
        reg.register(0, 0, 0, 0, 2, 0);   // addr=0同上
        reg.register(6, 0, 6, 0, 3, 0);   // series=1,col=0 → 不同 col

        // 修正：颜色 idx=0 和 idx=6 的 col 不同（0 vs 0），series 不同
        // 触发同一 col=0 的公约: idx=0, idx=6
        // 需要第3个 col=0: idx=12 → col=0 (series=2,col=0=12%7? no, 12//7=1,12%7=5)
        // 42色 ÷ 7列 → col = idx % 7
        // col=0 的 idx: 0, 7, 14, 21, 28, 35
        let mut reg2 = Registry::new();
        reg2.register(0, 0, 0, 0, 0, 0);   // idx=0, col=0
        reg2.register(7, 0, 7, 1, 1, 0);    // idx=7, col=0
        reg2.register(14, 0, 14, 2, 2, 0);  // idx=14, col=0

        let cid = reg2.check_convention(0, 0);
        assert_ne!(cid, 0xFFFFFFFF);
        assert_eq!(reg2.convention_count, 1);
    }
}
