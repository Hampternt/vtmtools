//! Wealth-tracking subsystem shared types.
//!
//! See `docs/superpowers/specs/2026-05-20-wealth-tracking-design.md`.

use serde::{Deserialize, Serialize};

// ─── Buckets / time ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Bucket {
    Liquid,
    Banked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimeUnit {
    Minutes,
    Hours,
    Days,
    Weeks,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransferDelay {
    pub value: f64,
    pub unit: TimeUnit,
}

// ─── Wallet ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Wallet {
    pub character_id: i64,
    pub liquid: f64,
    pub banked: f64,
    pub currency_name: String,
    pub transfer_delay: Option<TransferDelay>,
    pub created_at: String,
    pub updated_at: String,
}

// ─── Asset categories / conditions ─────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetCategory {
    Weapon,
    Vehicle,
    Property,
    Investment,
    Misc,
}

impl AssetCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssetCategory::Weapon     => "weapon",
            AssetCategory::Vehicle    => "vehicle",
            AssetCategory::Property   => "property",
            AssetCategory::Investment => "investment",
            AssetCategory::Misc       => "misc",
        }
    }

    pub fn from_str(s: &str) -> Option<AssetCategory> {
        Some(match s {
            "weapon"     => AssetCategory::Weapon,
            "vehicle"    => AssetCategory::Vehicle,
            "property"   => AssetCategory::Property,
            "investment" => AssetCategory::Investment,
            "misc"       => AssetCategory::Misc,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetCondition {
    New,
    Used,
    VeryUsed,
    Custom,
}

impl AssetCondition {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssetCondition::New      => "new",
            AssetCondition::Used     => "used",
            AssetCondition::VeryUsed => "very_used",
            AssetCondition::Custom   => "custom",
        }
    }

    pub fn from_str(s: &str) -> Option<AssetCondition> {
        Some(match s {
            "new"       => AssetCondition::New,
            "used"      => AssetCondition::Used,
            "very_used" => AssetCondition::VeryUsed,
            "custom"    => AssetCondition::Custom,
            _ => return None,
        })
    }
}

/// Condition multiplier lookup. `Custom` is GM-supplied at purchase time and
/// not stored here.
pub const CONDITION_MULTIPLIER_NEW:       f64 = 1.0;
pub const CONDITION_MULTIPLIER_USED:      f64 = 0.6;
pub const CONDITION_MULTIPLIER_VERY_USED: f64 = 0.3;

// ─── Custom modifiers ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CustomModifier {
    pub label: String,
    pub mult: f64,
}

// ─── Asset ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: i64,
    pub character_id: i64,
    pub name: String,
    pub category: Option<AssetCategory>,
    pub class: Option<String>,
    pub purchase_price: Option<f64>,
    pub current_value: f64,
    pub condition: Option<AssetCondition>,
    pub custom_modifiers: Vec<CustomModifier>,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewAsset {
    pub name: String,
    pub category: Option<AssetCategory>,
    pub class: Option<String>,
    pub purchase_price: Option<f64>,
    pub current_value: f64,
    pub condition: Option<AssetCondition>,
    pub custom_modifiers: Vec<CustomModifier>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetPatch {
    pub name: Option<String>,
    pub category: Option<Option<AssetCategory>>,
    pub class: Option<Option<String>>,
    pub purchase_price: Option<Option<f64>>,
    pub current_value: Option<f64>,
    pub condition: Option<Option<AssetCondition>>,
    pub custom_modifiers: Option<Vec<CustomModifier>>,
    pub notes: Option<Option<String>>,
}

// ─── Item slot (catalog) ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemSlot {
    pub id: i64,
    pub category: AssetCategory,
    pub class: String,
    pub label: String,
    pub base_price_min: f64,
    pub base_price_max: f64,
    pub examples: Option<String>,
    pub is_custom: bool,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewItemSlot {
    pub category: AssetCategory,
    pub class: String,
    pub label: String,
    pub base_price_min: f64,
    pub base_price_max: f64,
    pub examples: Option<String>,
    pub source: Option<String>,
}

// ─── Composed transaction requests ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseRequest {
    pub character_id: i64,
    pub slot_id: i64,
    pub price: f64,
    pub condition: AssetCondition,
    pub custom_modifiers: Vec<CustomModifier>,
    pub name_override: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SellRequest {
    pub asset_id: i64,
    pub sale_price: f64,
    pub notes: Option<String>,
}
