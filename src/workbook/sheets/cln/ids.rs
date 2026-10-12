//! Marker slugs (`tblLabs.marker_id`) the clinical views read. Slugs of markers
//! David has measured come from biomarker-cli; the rest (insulin, Lp(a), ferritin...)
//! are the slugs the views expect once those markers arrive. Change a slug here and
//! every Cln_* formula follows.

pub const TC: &str = "total-cholesterol";
pub const HDL: &str = "hdl-c";
pub const LDL: &str = "ldl-c";
pub const TG: &str = "triglycerides";
pub const APOB: &str = "apob";
pub const VLDL: &str = "vldl-c";
pub const CHOL_HDL: &str = "chol-hdl-ratio";
pub const LPA: &str = "lpa";
pub const GLUCOSE: &str = "glucose";
pub const A1C: &str = "hba1c";
pub const EAG: &str = "estimated-average-glucose";
pub const INSULIN: &str = "insulin";
pub const SODIUM: &str = "sodium";
pub const CHLORIDE: &str = "chloride";
pub const CO2: &str = "co2";
pub const ANION_GAP: &str = "anion-gap";
pub const ALBUMIN: &str = "albumin";
pub const GLOBULIN: &str = "globulin";
pub const AG_RATIO: &str = "a-g-ratio";
pub const TOTAL_PROTEIN: &str = "total-protein";
pub const CREATININE: &str = "creatinine";
pub const EGFR: &str = "egfr";
pub const UACR: &str = "uacr";
pub const BUN: &str = "bun";
pub const BUN_CR: &str = "bun-creatinine-ratio";
pub const CALCIUM: &str = "calcium";
pub const ALT: &str = "alt";
pub const AST: &str = "ast";
pub const PLATELETS: &str = "platelets";
pub const HEMOGLOBIN: &str = "hemoglobin";
pub const MCV: &str = "mcv";
pub const RDW: &str = "rdw";
pub const RBC: &str = "rbc";
pub const NEUTROPHILS: &str = "neutrophils-abs";
pub const LYMPHOCYTES: &str = "lymphocytes-abs";
pub const FERRITIN: &str = "ferritin";
pub const IRON: &str = "iron";
pub const TIBC: &str = "tibc";
pub const TSH: &str = "tsh";
pub const FT4: &str = "ft4";
pub const URIC: &str = "uric-acid";
pub const HOMOCYSTEINE: &str = "homocysteine";
pub const CRP: &str = "hs-crp";
pub const PREALBUMIN: &str = "prealbumin";
pub const TRANSFERRIN: &str = "transferrin";
pub const VITAMIN_D: &str = "vitamin-d";
pub const B12: &str = "vitamin-b12";
