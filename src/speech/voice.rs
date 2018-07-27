pub struct Font {
    pub lang: &'static str,
    pub gender: &'static str,
    pub name: &'static str,
}

macro_rules! font {
    ($var:ident { $lang:expr, $gender:expr, $name:expr }) => {
        pub const $var: &'static super::Font = &super::Font {
            lang: $lang,
            gender: $gender,
            name: $name,
        };
    };
}

pub mod ar_eg {
    font!(HODA {
        "ar-EG",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (ar-EG, Hoda)"
    });
}

pub mod ar_sa {
    font!(NAAYF {
        "ar-SA",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (ar-SA, Naayf)"
    });
}

pub mod bg_bg {
    font!(IVAN {
        "bg-BG",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (bg-BG, Ivan)"
    });
}

pub mod ca_es {
    font!(HERENA_RUS {
        "ca-ES",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (ca-ES, HerenaRUS)"
    });
}

pub mod ca_cz {
    font!(JAKUB {
        "ca-CZ",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (cs-CZ, Jakub)"
    });
}

pub mod da_dk {
    font!(HELLE_RUS {
        "da-DK",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (da-DK, HelleRUS)"
    });
}

pub mod de_at {
    font!(MICHAEL {
        "de-AT",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (de-AT, Michael)"
    });
}

pub mod de_ch {
    font!(KARSTEN {
        "de-CH",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (de-CH, Karsten)"
    });
}

pub mod de_de {
    font!(HEDDA {
        "de-DE",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (de-DE, Hedda)"
    });

    font!(HEDDA_RUS {
        "de-DE",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (de-DE, HeddaRUS)"
    });

    font!(STEFAN_APOLLO {
        "de-DE",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (de-DE, Stefan, Apollo)"
    });
}

pub mod el_gr {
    font!(STEFANOS {
        "el-GR",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (el-GR, Stefanos)"
    });
}

pub mod en_au {
    font!(CATHERINE {
        "en-AU",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (en-AU, Catherine)"
    });

    font!(HAYLEYRUS {
        "en-AU",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (en-AU, HayleyRUS)"
    });
}

pub mod en_ca {
    font!(LINDA {
        "en-CA",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (en-CA, Linda)"
    });

    font!(HEATHER_RUS {
        "en-CA",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (en-CA, HeatherRUS)"
    });
}

pub mod en_gb {
    font!(SUSAN_APOLLO {
        "en-GB",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (en-GB, Susan, Apollo)"
    });

    font!(HAZEL_RUS {
        "en-GB",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (en-GB, HazelRUS)"
    });

    font!(GEORGE_APOLLO {
        "en-GB",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (en-GB, George, Apollo)"
    });
}

pub mod en_ie {
    font!(SEAN {
        "en-IE",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (en-IE, Sean)"
    });
}

pub mod en_in {
    font!(HEERA_APOLLO {
        "en-IN",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (en-IN, Heera, Apollo)"
    });

    font!(PRIYA_RUS {
        "en-IN",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (en-IN, PriyaRUS)"
    });

    font!(RAVI_APOLLO {
        "en-IN",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (en-IN, Ravi, Apollo)"
    });
}

pub mod en_us {
    font!(ZIRA_RUS {
        "en-US",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (en-US, ZiraRUS)"
    });

    font!(JESSA_RUS {
        "en-US",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (en-US, JessaRUS)"
    });

    font!(BENJAMIN_RUS {
        "en-US",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (en-US, BenjaminRUS)"
    });
}

pub mod es_es {
    font!(LAURA_APOLLO {
        "es-ES",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (es-ES, Laura, Apollo)"
    });

    font!(HELENA_RUS {
        "es-ES",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (es-ES, HelenaRUS)"
    });

    font!(PABLO_APOLLO {
        "es-ES",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (es-ES, Pablo, Apollo)"
    });
}

pub mod es_mx {
    font!(HILDA_RUS {
        "es-MX",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (es-MX, HildaRUS)"
    });

    font!(RAUL_APOLLO {
        "es-MX",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (es-MX, Raul, Apollo)"
    });
}

pub mod fi_fi {
    font!(HEIDI_RUS {
        "fi-FI",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (fi-FI, HeidiRUS)"
    });
}

pub mod fr_ca {
    font!(CAROLINE {
        "fr-CA",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (fr-CA, Caroline)"
    });

    font!(HARMONIE_RUS {
        "fr-CA",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (fr-CA, HarmonieRUS)"
    });
}

pub mod fr_ch {
    font!(GUILLAUME {
        "fr-CH",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (fr-CH, Guillaume)"
    });
}

pub mod fr_fr {
    font!(JULIE_APOLLO {
        "fr-FR",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (fr-FR, JulieApollo)"
    });

    font!(HORTENSE_RUS {
        "fr-FR",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (fr-FR, HortenseRUS)"
    });

    font!(PAUL_APOLLO {
        "fr-FR",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (fr-FR, PaulApollo)"
    });
}

pub mod he_il {
    font!(ASAF {
        "he-IL",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (he-IL, Asaf)"
    });
}

pub mod hi_in {
    font!(KALPANA_APOLLO {
        "hi-IN",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (hi-IN, Kalpana, Apollo)"
    });

    font!(KALPANA {
        "hi-IN",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (hi-IN, Kalpana)"
    });

    font!(HEMANT {
        "hi-IN",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (hi-IN, Hemant)"
    });
}

pub mod hr_hr {
    font!(MATEJ {
        "hr-HR",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (hr-HR, Matej)"
    });
}

pub mod hu_hu {
    font!(SZABOLCS {
        "hu-HU",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (hu-HU, Szabolcs)"
    });
}

pub mod id_id {
    font!(ANDIKA {
        "id-ID",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (id-ID, Andika)"
    });
}

pub mod it_it {
    font!(COSIMA_APOLLO {
        "it-IT",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (it-IT, Cosimo, Apollo)"
    });
}

pub mod ja_jp {
    font!(AYUMI_APOLLO {
        "ja-JP",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (ja-JP, Ayumi, Apollo)"
    });

    font!(ICHIRO_APOLLO {
        "ja-JP",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (ja-JP, Ichiro, Apollo)"
    });

    font!(HARUKA_RUS {
        "ja-JP",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (ja-JP, HarukaRUS)"
    });

    font!(LUCIA_RUS {
        "ja-JP",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (ja-JP, LuciaRUS)"
    });

    font!(EKATERINA_RUS {
        "ja-JP",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (ja-JP, EkaterinaRUS)"
    });
}

pub mod ko_kr {
    font!(HEAMI_RUS {
        "ko-KR",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (ko-KR, HeamiRUS)"
    });
}

pub mod ms_my {
    font!(RIZWAN {
        "ms-MY",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (ms-MY, Rizwan)"
    });
}

pub mod nb_no {
    font!(HULDA_RUS {
        "nb-NO",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (nb-NO, HuldaRUS)"
    });
}

pub mod nl_nl {
    font!(HANNA_RUS {
        "nl-NL",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (nl-NL, HannaRUS)"
    });
}

pub mod pl_pl {
    font!(PAULINA_RUS {
        "pl-PL",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (pl-PL, PaulinaRUS)"
    });
}

pub mod pt_br {
    font!(HELOISA_RUS {
        "pt-BR",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (pt-BR, HeloisaRUS)"
    });

    font!(DANIEL_APOLLO {
        "pt-BR",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (pt-BR, DanielApollo)"
    });
}

pub mod pt_pt {
    font!(HELIA_RUS {
        "pt-PT",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (pt-PT, HeliaRUS)"
    });
}

pub mod ro_ro {
    font!(ANDREI {
        "ro-RO",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (ro-RO, Andrei)"
    });
}

pub mod ru_ru {
    font!(IRINA_APOLLO {
        "ru-RU",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (ru-RU, Irina, Apollo)"
    });

    font!(PAVEL_APOLLO {
        "ru-RU",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (ru-RU, Pavel, Apollo)"
    });
}

pub mod sk_sk {
    font!(FILIP {
        "sk-SK",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (sk-SK, Filip)"
    });
}

pub mod sl_si {
    font!(LADO {
        "sl-SI",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (sl-SI, Lado)"
    });
}

pub mod sv_se {
    font!(HEDVIG_RUS {
        "sv-SE",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (sv-SE, HedvigRUS)"
    });
}

pub mod ta_in {
    font!(VALLUVAR {
        "ta-IN",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (ta-IN, Valluvar)"
    });
}

pub mod th_th {
    font!(PATTARA {
        "th-TH",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (th-TH, Pattara)"
    });
}

pub mod tr_tr {
    font!(SEDA_RUS {
        "tr-TR",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (tr-TR, SedaRUS)"
    });
}

pub mod vi_vn {
    font!(AN {
        "vi-VN",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (vi-VN, An)"
    });
}

pub mod zh_cn {
    font!(HUIHUI_RUS {
        "zh-CN",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (zh-CN, HuihuiRUS)"
    });

    font!(YAOYAO_APOLLO {
        "zh-CN",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (zh-CN, Yaoyao, Apollo)"
    });

    font!(KANGKANG_APOLLO {
        "zh-CN",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (zh-CN, Kangkang, Apollo)"
    });
}

pub mod zh_hk {
    font!(TRACY_APOLLO {
        "zh-HK",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (zh-HK, Tracy, Apollo)"
    });

    font!(TRACY_RUS {
        "zh-HK",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (zh-HK, TracyRUS)"
    });

    font!(DANNY_APOLLO {
        "zh-HK",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (zh-HK, Danny, Apollo)"
    });
}

pub mod zh_tw {
    font!(YATING_APOLLO {
        "zh-TW",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (zh-TW, Yating, Apollo)"
    });

    font!(HANHAN_RUS {
        "zh-TW",
        "Female",
        "Microsoft Server Speech Text to Speech Voice (zh-TW, HanHanRUS)"
    });

    font!(ZHIWEI_APOLLO {
        "zh-TW",
        "Male",
        "Microsoft Server Speech Text to Speech Voice (zh-TW, Zhiwei, Apollo)"
    });
}
