/// A non-rejecting `bsqr:maxLength` advisory for one transported field.
///
/// `actual_character_count` describes the field's QR-sequence representation.
/// It uses Unicode scalar-value counts for textual values and counts compacted
/// transport representations for values such as dates and classifiers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdvisoryDiagnostic {
    pub field_path: String,
    pub actual_character_count: usize,
    pub recommended_maximum: usize,
}
