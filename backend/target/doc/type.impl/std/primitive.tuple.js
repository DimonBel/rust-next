(function() {
    var type_impls = Object.fromEntries([["docx_rs",[]],["lopdf",[]],["pdf_extract",[]],["postscript",[]],["xml",[]]]);
    if (window.register_type_impls) {
        window.register_type_impls(type_impls);
    } else {
        window.pending_type_impls = type_impls;
    }
})()
//{"start":55,"fragment_lengths":[14,13,19,18,11]}