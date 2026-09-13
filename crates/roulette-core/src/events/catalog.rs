//! Static catalog and registry of game events.

use roulette_domain::{EventId, EventTier};

/// Static definition and metadata of an event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventDef {
    /// Unique stable identifier indexing this event.
    pub id: EventId,
    /// Dramatic rarity tier.
    pub tier: EventTier,
    /// User-facing headline title.
    pub title: &'static str,
    /// Narrative summary describing the flavor and physical fact.
    pub narrative: &'static str,
    /// Branch path depth cost (0 for transitions/dampeners, 1-4 for higher chains).
    pub chain_cost: u32,
}

/// Global registry of known events.
pub struct EventRegistry;

impl EventRegistry {
    /// Returns the static event definition for the given event ID.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn get(id: &EventId) -> Option<EventDef> {
        match id.0.as_str() {
            "evt_weather_blizzard" => Some(EventDef {
                id: EventId::new("evt_weather_blizzard"),
                tier: EventTier::Epic,
                title: "极寒暴雪",
                narrative: "刺骨暴雪骤降，狂风呼啸，全图水面瞬间凝结成坚冰！",
                chain_cost: 3,
            }),
            "evt_water_freeze_ice" => Some(EventDef {
                id: EventId::new("evt_water_freeze_ice"),
                tier: EventTier::Uncommon,
                title: "水面凝结",
                narrative: "严寒之下水波停滞，迅速冻结为光滑冰面。",
                chain_cost: 0,
            }),
            "evt_disoriented_reverse_shot" => Some(EventDef {
                id: EventId::new("evt_disoriented_reverse_shot"),
                tier: EventTier::Uncommon,
                title: "晕头转向",
                narrative: "射手视野昏花，在恍惚中竟然朝相反方向扣动了扳机！",
                chain_cost: 1,
            }),
            "evt_revolver_misfire" => Some(EventDef {
                id: EventId::new("evt_revolver_misfire"),
                tier: EventTier::Normal,
                title: "撞针空响",
                narrative: "转轮枪发出清脆的卡嗒声，哑火了，现场一片死寂。",
                chain_cost: 0,
            }),
            "evt_crate_splinter_blast" => Some(EventDef {
                id: EventId::new("evt_crate_splinter_blast"),
                tier: EventTier::Rare,
                title: "木箱爆碎",
                narrative: "木箱瞬间四分五裂，飞溅的破片与冲击波扩散向四周！",
                chain_cost: 2,
            }),
            "evt_splinter_scratch" => Some(EventDef {
                id: EventId::new("evt_splinter_scratch"),
                tier: EventTier::Normal,
                title: "飞屑划伤",
                narrative: "爆裂的飞屑击中附近隐蔽的玩家，引起一阵惊呼。",
                chain_cost: 1,
            }),
            "evt_ice_slide" => Some(EventDef {
                id: EventId::new("evt_ice_slide"),
                tier: EventTier::Uncommon,
                title: "冰面滑行",
                narrative: "踏上光滑的冰面，身形不受控制地顺势向前滑行了一格！",
                chain_cost: 1,
            }),
            "evt_mine_chain_detonation" => Some(EventDef {
                id: EventId::new("evt_mine_chain_detonation"),
                tier: EventTier::Legendary,
                title: "地壳震荡",
                narrative: "猛烈的爆炸冲击波撕裂地表，波及周边区域！",
                chain_cost: 4,
            }),
            "evt_dust_settles" => Some(EventDef {
                id: EventId::new("evt_dust_settles"),
                tier: EventTier::Normal,
                title: "烟尘落定",
                narrative: "激荡的冲击波渐渐平息，硝烟与碎屑归于沉寂。",
                chain_cost: 0,
            }),
            "evt_recoil_knockback" => Some(EventDef {
                id: EventId::new("evt_recoil_knockback"),
                tier: EventTier::Rare,
                title: "后坐力强冲",
                narrative: "转轮手枪超量装药！强大的后坐力将射手反冲倒退一格！",
                chain_cost: 2,
            }),
            "evt_piercing_slug" => Some(EventDef {
                id: EventId::new("evt_piercing_slug"),
                tier: EventTier::Epic,
                title: "穿甲重弹",
                narrative: "高温穿甲弹带着刺耳呼啸，贯穿了前方掩体！",
                chain_cost: 2,
            }),
            "evt_sprint_dash" => Some(EventDef {
                id: EventId::new("evt_sprint_dash"),
                tier: EventTier::Uncommon,
                title: "骤然突进",
                narrative: "脚步发力过猛，惯性带着身体顺势向前多冲刺了一格！",
                chain_cost: 1,
            }),
            "evt_stumble_trip" => Some(EventDef {
                id: EventId::new("evt_stumble_trip"),
                tier: EventTier::Uncommon,
                title: "脚底绊蒜",
                narrative: "踩到碎石脚下一滑狼狈摔倒，未能移动，呆立原地！",
                chain_cost: 1,
            }),
            "evt_spatial_swap" => Some(EventDef {
                id: EventId::new("evt_spatial_swap"),
                tier: EventTier::Legendary,
                title: "空间对调",
                narrative: "强烈的地磁混乱撕裂空间，玩家与地图上一处随机实体对调了位置！",
                chain_cost: 3,
            }),
            "evt_crate_surprise_mine" => Some(EventDef {
                id: EventId::new("evt_crate_surprise_mine"),
                tier: EventTier::Epic,
                title: "箱中藏雷",
                narrative: "木箱破裂的瞬间引爆了藏在底部的触发式地雷！",
                chain_cost: 3,
            }),
            "evt_crate_surprise_medkit" => Some(EventDef {
                id: EventId::new("evt_crate_surprise_medkit"),
                tier: EventTier::Uncommon,
                title: "翻出护盾",
                narrative: "木箱散落开来，里面掉落出了一件崭新的防护单兵盾！",
                chain_cost: 1,
            }),
            "evt_ice_crack_collapse" => Some(EventDef {
                id: EventId::new("evt_ice_crack_collapse"),
                tier: EventTier::Rare,
                title: "薄冰碎裂",
                narrative: "薄冰承受不住重量轰然破碎，冰面化作深水，玩家落入水中！",
                chain_cost: 2,
            }),
            "evt_weather_heatwave" => Some(EventDef {
                id: EventId::new("evt_weather_heatwave"),
                tier: EventTier::Epic,
                title: "炙热热浪",
                narrative: "滚滚热浪席卷战场，地图上所有的坚冰瞬间消融化为深水！",
                chain_cost: 2,
            }),
            "evt_meteor_strike" => Some(EventDef {
                id: EventId::new("evt_meteor_strike"),
                tier: EventTier::Legendary,
                title: "天降陨石",
                narrative: "一颗燃烧的天外陨石轰然砸中地表，摧毁了目标地貌！",
                chain_cost: 3,
            }),
            "evt_nothing_happens" => Some(EventDef {
                id: EventId::new("evt_nothing_happens"),
                tier: EventTier::Normal,
                title: "风平浪静",
                narrative: "周围空气略显沉寂，暂时没有发生意外。",
                chain_cost: 0,
            }),
            _ => None,
        }
    }
}
