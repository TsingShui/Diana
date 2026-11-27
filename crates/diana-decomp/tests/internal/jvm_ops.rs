use super::*;
use crate::input::jvm::bytecode::Code;

fn class_bytes() -> Vec<u8> {
    let mut bytes = vec![0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 61, 0, 7];
    for (name, name_index) in [
        (b"fixture/Ops".as_slice(), 1),
        (b"java/lang/Object".as_slice(), 3),
        (b"[[I".as_slice(), 5),
    ] {
        bytes.push(1);
        bytes.extend((name.len() as u16).to_be_bytes());
        bytes.extend(name);
        bytes.extend([7, 0, name_index]);
    }
    bytes.extend([0, 0x21, 0, 2, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0]);
    bytes
}

fn attribute(bytes: &[u8], max_stack: u16, max_locals: u16) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend(max_stack.to_be_bytes());
    body.extend(max_locals.to_be_bytes());
    body.extend((bytes.len() as u32).to_be_bytes());
    body.extend(bytes);
    body.extend([0; 4]);
    body
}

#[test]
fn transfers_wide_stack_permutation_before_ordered_long_arithmetic() {
    let bytes = class_bytes();
    let class = ClassFile::parse(&bytes).unwrap();
    // [i, j] -> [j, i, j] -> [j, i] -> [j, (long) i] -> [j+(long)i].
    let body = attribute(&[0x1a, 0x1f, 0x5d, 0x58, 0x85, 0x61, 0xad], 5, 3);
    let registers = Registers::new(3, 5, 3).unwrap();
    let mut stack = Stack::default();
    stack.seed_local(0, ArgType::INT, None).unwrap();
    stack.seed_local(1, ArgType::LONG, None).unwrap();
    let mut output = Vec::new();
    for insn in Code::parse(&body).unwrap().instructions() {
        apply(
            &class,
            insn.unwrap(),
            &mut stack,
            registers,
            Some(&mut output),
        )
        .unwrap();
    }
    let scratch = output
        .iter()
        .filter(|insn| insn.offset == 2)
        .map(|insn| (insn.args[0].reg_num().unwrap(), insn.result_reg().unwrap()))
        .collect::<Vec<_>>();
    assert_eq!(scratch, [(3, 8), (4, 9), (9, 3), (8, 5), (9, 6)]);
    let add = output.iter().find(|insn| insn.offset == 5).unwrap();
    assert_eq!(add.insn_type, InsnType::Arith);
    assert_eq!(add.payload.arith_op, Some(ArithOp::Add));
    assert_eq!(add.args[0].reg_num(), Some(3));
    assert_eq!(add.args[1].reg_num(), Some(5));
    assert_eq!(add.result.as_ref().unwrap().ty, ArgType::LONG);
    assert_eq!(output.last().unwrap().args[0].reg_num(), Some(3));
}

#[test]
fn multidimensional_allocation_keeps_original_dimensions_and_consumers() {
    let bytes = class_bytes();
    let class = ClassFile::parse(&bytes).unwrap();
    // Store the allocated array, load it, discard it and return -8.
    let body = attribute(
        &[
            0x04, 0x05, 0xc5, 0, 6, 2, 0x4b, 0x2a, 0x57, 0x10, 0xf8, 0xac,
        ],
        2,
        1,
    );
    let registers = Registers::new(1, 2, 0).unwrap();
    let mut stack = Stack::default();
    let mut output = Vec::new();
    for item in Code::parse(&body).unwrap().instructions() {
        apply(
            &class,
            item.unwrap(),
            &mut stack,
            registers,
            Some(&mut output),
        )
        .unwrap();
    }
    let offsets = output.iter().map(|insn| insn.offset).collect::<Vec<_>>();
    assert_eq!(offsets, [0, 1, 2, 6, 7, 9, 11]);
    let allocation = &output[2];
    assert_eq!(allocation.insn_type, InsnType::NewArray);
    assert_eq!(allocation.args.len(), 2);
    assert_eq!(allocation.args[0].reg_num(), output[0].result_reg());
    assert_eq!(allocation.args[1].reg_num(), output[1].result_reg());
    assert_eq!(
        allocation.result.as_ref().unwrap().ty.to_descriptor(),
        "[[I"
    );
    assert_eq!(output[3].args[0].reg_num(), allocation.result_reg());
    assert_eq!(output[4].args[0].reg_num(), output[3].result_reg());
    assert_eq!(output[5].args[0].const_value(), Some(-8));
    assert_eq!(output[6].insn_type, InsnType::Return);
    assert_eq!(output[6].args[0].reg_num(), output[5].result_reg());
}

#[test]
fn widens_reference_and_int_family_slots_with_stable_stack_shape() {
    let registers = Registers::new(4, 3, 2).unwrap();
    let mut left = Stack::default();
    let mut right = Stack::default();
    left.push(Value::new(ArgType::string(), true).unwrap(), registers)
        .unwrap();
    right
        .push(
            Value::new(ArgType::array(ArgType::INT), true).unwrap(),
            registers,
        )
        .unwrap();
    left.push(Value::new(ArgType::BOOLEAN, true).unwrap(), registers)
        .unwrap();
    right
        .push(Value::new(ArgType::INT, false).unwrap(), registers)
        .unwrap();
    assert!(left.join(&right).unwrap());
    assert!(!left.join(&right).unwrap());
    assert_eq!(left.slots[0].ty, ArgType::unknown_object());
    assert_eq!(left.slots[1].ty, ArgType::INT);
    assert!(!left.slots[1].available);

    assert_eq!(registers.local_width(1, 2).unwrap(), 1);
    assert_eq!(registers.local(2).unwrap(), 2);
    assert_eq!(registers.input_register(1).unwrap(), registers.count() - 1);
    assert_eq!(registers.stack(0), 4);
}

#[test]
fn exception_keeps_known_locals_and_replaces_the_ordinary_stack() {
    let registers = Registers::new(4, 4, 1).unwrap();
    let mut stack = Stack::default();
    stack
        .seed_local(0, ArgType::object("fixture/Ops"), None)
        .unwrap();
    stack
        .push(Value::new(ArgType::LONG, true).unwrap(), registers)
        .unwrap();
    stack
        .push_exception(ArgType::throwable(), registers)
        .unwrap();
    assert_eq!(stack.words, 1);
    assert_eq!(
        stack.local_value(0, Category::Reference).unwrap().ty,
        ArgType::object("fixture/Ops")
    );
    let exception = stack.pop(registers, Some(Category::Reference)).unwrap();
    assert_eq!(exception.register, registers.stack(0));
    assert_eq!(exception.value.ty, ArgType::throwable());
}

/// The indices and contents of these class-file rows are independent of the producer.
#[derive(Default)]
struct Pool {
    items: Vec<Vec<u8>>,
}
impl Pool {
    fn item(&mut self, tag: u8, data: &[u8]) -> u16 {
        let mut row = vec![tag];
        row.extend(data);
        self.items.push(row);
        self.items.len() as u16
    }

    fn words(&mut self, tag: u8, values: &[u16]) -> u16 {
        self.item(
            tag,
            &values
                .iter()
                .flat_map(|n| n.to_be_bytes())
                .collect::<Vec<_>>(),
        )
    }

    fn utf8(&mut self, value: &[u8]) -> u16 {
        let mut row = (value.len() as u16).to_be_bytes().to_vec();
        row.extend(value);
        self.item(1, &row)
    }

    fn class(&mut self, value: &[u8]) -> u16 {
        let name = self.utf8(value);
        self.words(7, &[name])
    }

    fn string(&mut self, value: &[u8]) -> u16 {
        let value = self.utf8(value);
        self.words(8, &[value])
    }

    fn method(&mut self, owner: &[u8], name: &[u8], descriptor: &[u8]) -> u16 {
        let owner = self.class(owner);
        let name = self.utf8(name);
        let descriptor = self.utf8(descriptor);
        let signature = self.words(12, &[name, descriptor]);
        self.words(10, &[owner, signature])
    }

    fn handle(&mut self, kind: u8, reference: u16) -> u16 {
        let [upper, lower] = reference.to_be_bytes();
        self.item(15, &[kind, upper, lower])
    }

    fn dynamic(&mut self, name: &[u8], descriptor: &[u8]) -> u16 {
        let name = self.utf8(name);
        let descriptor = self.utf8(descriptor);
        let signature = self.words(12, &[name, descriptor]);
        self.words(18, &[0, signature])
    }

    fn finish(mut self, function: u16, arguments: &[u16], methods: &[[u16; 4]]) -> Vec<u8> {
        let this = self.class(b"fixture/Ops");
        let parent = self.class(b"java/lang/Object");
        let bootstrap_name = self.utf8(b"BootstrapMethods");
        let mut bytes = vec![0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 61];
        bytes.extend((self.items.len() as u16 + 1).to_be_bytes());
        bytes.extend(self.items.into_iter().flatten());
        for word in [0x0021, this, parent, 0, 0, methods.len() as u16]
            .into_iter()
            .chain(methods.iter().flatten().copied())
            .chain([1, bootstrap_name])
        {
            bytes.extend(word.to_be_bytes());
        }
        bytes.extend((6 + 2 * arguments.len() as u32).to_be_bytes());
        for word in [1, function, arguments.len() as u16]
            .into_iter()
            .chain(arguments.iter().copied())
        {
            bytes.extend(word.to_be_bytes());
        }
        bytes
    }
}

#[test]
fn concat_recipe_preserves_dynamic_order_and_original_utf16_literals() {
    let mut pool = Pool::default();
    let bootstrap = pool.method(
        b"java/lang/invoke/StringConcatFactory",
        b"makeConcatWithConstants",
        b"(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/invoke/MethodType;Ljava/lang/String;[Ljava/lang/Object;)Ljava/lang/invoke/CallSite;",
    );
    let handle = pool.handle(6, bootstrap);
    let dynamic = pool.dynamic(b"requested", b"(ILjava/lang/String;)Ljava/lang/String;");
    let recipe = pool.string(b"[\x01\x02\x01:\xed\xa0\xbc\xed\xbc\xb1\xc0\x80");
    let constant = pool.string(b"]\x01 ");
    let bytes = pool.finish(handle, &[recipe, constant], &[]);
    let class = ClassFile::parse(&bytes).unwrap();
    let [upper, lower] = dynamic.to_be_bytes();
    let body = attribute(&[0x1a, 0x2b, 0xba, upper, lower, 0, 0, 0xb0], 2, 2);
    let mut frame = Stack::default();
    frame.seed_local(0, ArgType::INT, None).unwrap();
    frame.seed_local(1, ArgType::string(), None).unwrap();
    let registers = Registers::new(2, 2, 2).unwrap();
    let mut output = Vec::new();
    for item in Code::parse(&body).unwrap().instructions() {
        apply(
            &class,
            item.unwrap(),
            &mut frame,
            registers,
            Some(&mut output),
        )
        .unwrap();
    }
    let concat = &output[2];
    assert_eq!(concat.insn_type, InsnType::StringConcat);
    assert_eq!(concat.args.len(), 5);
    let units = |position: usize| match &concat.args[position] {
        InsnArg::Wrapped(literal) => literal.payload.string_value.as_ref().unwrap().as_utf16(),
        _ => panic!("expected original UTF-16 literal"),
    };
    assert_eq!(units(0), &[u16::from(b'[')]);
    assert_eq!(concat.args[1].reg_num(), output[0].result_reg());
    assert_eq!(units(2), &[u16::from(b']'), 1, u16::from(b' ')]);
    assert_eq!(concat.args[3].reg_num(), output[1].result_reg());
    assert_eq!(units(4), &[u16::from(b':'), 0xd83c, 0xdf31, 0]);
}

#[test]
fn simple_factory_recovers_exact_primitive_conversions_and_marks_object_conversion_unknown() {
    for (first_type, first_descriptor, expected) in [
        (
            ArgType::INT,
            b"(IJ)Ljava/lang/String;".as_slice(),
            InsnType::StringConcat,
        ),
        (
            ArgType::object("java/lang/Object"),
            b"(Ljava/lang/Object;J)Ljava/lang/String;".as_slice(),
            InsnType::Unavailable,
        ),
    ] {
        let mut pool = Pool::default();
        let bootstrap = pool.method(
            b"java/lang/invoke/StringConcatFactory",
            b"makeConcat",
            b"(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/invoke/MethodType;)Ljava/lang/invoke/CallSite;",
        );
        let handle = pool.handle(6, bootstrap);
        let dynamic = pool.dynamic(b"called", first_descriptor);
        let bytes = pool.finish(handle, &[], &[]);
        let class = ClassFile::parse(&bytes).unwrap();
        let [upper, lower] = dynamic.to_be_bytes();
        let first = if first_type == ArgType::INT {
            0x1a
        } else {
            0x2a
        };
        let body = attribute(&[first, 0x1f, 0xba, upper, lower, 0, 0, 0xb0], 3, 3);
        let registers = Registers::new(3, 3, 3).unwrap();
        let mut frame = Stack::default();
        frame.seed_local(0, first_type, None).unwrap();
        frame.seed_local(1, ArgType::LONG, None).unwrap();
        let mut output = Vec::new();
        let mut issues = Vec::new();
        for item in Code::parse(&body).unwrap().instructions() {
            if let Err(error) = apply(
                &class,
                item.unwrap(),
                &mut frame,
                registers,
                Some(&mut output),
            ) {
                assert!(error.retains_stack());
                issues.push(error);
            }
        }
        assert_eq!(output[2].insn_type, expected);
        assert_eq!(output[3].args[0].reg_num(), output[2].result_reg());
        if expected == InsnType::Unavailable {
            assert_eq!(issues.len(), 1);
            assert!(issues[0]
                .to_string()
                .contains("called(Ljava/lang/Object;J)Ljava/lang/String;"));
            assert_eq!(output[2].args[0].reg_num(), output[0].result_reg());
            assert_eq!(output[2].args[1].reg_num(), output[1].result_reg());
        } else {
            assert!(issues.is_empty());
            assert_eq!(output[2].args[1].reg_num(), output[0].result_reg());
            assert_eq!(output[2].args[2].reg_num(), output[1].result_reg());
        }
    }
}

#[test]
fn lambda_requires_one_original_local_static_method_with_unadapted_sam_types() {
    for instantiated in [b"(I)I".as_slice(), b"(J)I".as_slice()] {
        let mut pool = Pool::default();
        let bootstrap = pool.method(
            b"java/lang/invoke/LambdaMetafactory",
            b"metafactory",
            b"(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/invoke/MethodType;Ljava/lang/invoke/MethodType;Ljava/lang/invoke/MethodHandle;Ljava/lang/invoke/MethodType;)Ljava/lang/invoke/CallSite;",
        );
        let handle = pool.handle(6, bootstrap);
        let target = pool.method(b"fixture/Ops", b"square", b"(I)I");
        let target = pool.handle(6, target);
        let sam_text = pool.utf8(b"(I)I");
        let sam_type = pool.words(16, &[sam_text]);
        let declared = pool.utf8(b"square");
        let dynamic_type = pool.utf8(instantiated);
        let dynamic_type = pool.words(16, &[dynamic_type]);
        let dynamic = pool.dynamic(b"applyAsInt", b"()Ljava/util/function/IntUnaryOperator;");
        let bytes = pool.finish(
            handle,
            &[sam_type, target, dynamic_type],
            &[[0x0109, declared, sam_text, 0]],
        );
        let class = ClassFile::parse(&bytes).unwrap();
        let [upper, lower] = dynamic.to_be_bytes();
        let body = attribute(&[0xba, upper, lower, 0, 0, 0xb0], 1, 0);
        let registers = Registers::new(0, 1, 0).unwrap();
        let mut frame = Stack::default();
        let mut output = Vec::new();
        let mut issues = Vec::new();
        for item in Code::parse(&body).unwrap().instructions() {
            if let Err(error) = apply(
                &class,
                item.unwrap(),
                &mut frame,
                registers,
                Some(&mut output),
            ) {
                assert!(error.retains_stack());
                issues.push(error);
            }
        }
        assert_eq!(output[1].args[0].reg_num(), output[0].result_reg());
        if instantiated == b"(I)I" {
            assert!(issues.is_empty());
            assert_eq!(output[0].insn_type, InsnType::Invoke);
            let lambda = output[0].payload.lambda.as_ref().unwrap();
            assert_eq!(lambda.sam_name, "applyAsInt");
            assert_eq!(lambda.implementation.name, "square");
            assert_eq!(lambda.sam_descriptor.parameters, vec![ArgType::INT]);
        } else {
            assert_eq!(issues.len(), 1);
            assert_eq!(output[0].insn_type, InsnType::Unavailable);
        }
    }
}

#[test]
fn uninitialized_merged_local_becomes_an_explicit_typed_result_and_wide_overlap_invalidates() {
    let bytes = class_bytes();
    let class = ClassFile::parse(&bytes).unwrap();
    let registers = Registers::new(4, 2, 0).unwrap();
    let mut initialized = Stack::default();
    initialized.seed_local(1, ArgType::LONG, None).unwrap();
    assert!(initialized.local_value(2, Category::Long).is_none());
    initialized.seed_local(2, ArgType::INT, None).unwrap();
    assert!(initialized.local_value(1, Category::Long).is_none());
    initialized.seed_local(0, ArgType::INT, None).unwrap();

    let mut missing = Stack::default();
    missing.seed_local(2, ArgType::INT, None).unwrap();
    assert!(initialized.join(&missing).unwrap());
    assert!(!initialized.join(&missing).unwrap());
    assert_eq!(initialized.frame_values(), 1);
    let body = attribute(&[0x1a, 0x04, 0x60, 0xac], 2, 4);
    let mut output = Vec::new();
    let mut issues = Vec::new();
    for item in Code::parse(&body).unwrap().instructions() {
        if let Err(error) = apply(
            &class,
            item.unwrap(),
            &mut initialized,
            registers,
            Some(&mut output),
        ) {
            assert!(error.retains_stack());
            issues.push(error);
        }
    }
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].kind(), FailureKind::Malformed);
    assert!(issues[0].to_string().contains("local slot 0"));
    assert_eq!(output[0].insn_type, InsnType::Unavailable);
    assert!(output[0].args.is_empty());
    assert_eq!(output[0].result.as_ref().unwrap().ty, ArgType::INT);
    assert_eq!(output[2].insn_type, InsnType::Arith);
    assert_eq!(output[2].args[0].reg_num(), output[0].result_reg());
    assert_eq!(output[3].args[0].reg_num(), output[2].result_reg());
}

#[test]
fn bound_method_reference_captures_the_checked_receiver_once() {
    for (receiver, supported) in [
        (ArgType::object("fixture/Ops"), true),
        (ArgType::object("java/lang/Object"), false),
    ] {
        let mut pool = Pool::default();
        let bootstrap = pool.method(
            b"java/lang/invoke/LambdaMetafactory",
            b"metafactory",
            b"(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/invoke/MethodType;Ljava/lang/invoke/MethodType;Ljava/lang/invoke/MethodHandle;Ljava/lang/invoke/MethodType;)Ljava/lang/invoke/CallSite;",
        );
        let handle = pool.handle(6, bootstrap);
        let target = pool.method(b"fixture/Ops", b"mix", b"(J)I");
        let target = pool.handle(5, target);
        let sam_text = pool.utf8(b"(J)I");
        let sam_type = pool.words(16, &[sam_text]);
        let declared = pool.utf8(b"mix");
        let factory = receiver.to_descriptor();
        let factory = format!("({factory})Lfixture/LongToInt;");
        let dynamic = pool.dynamic(b"apply", factory.as_bytes());
        let bytes = pool.finish(
            handle,
            &[sam_type, target, sam_type],
            &[[0x0101, declared, sam_text, 0]],
        );
        let class = ClassFile::parse(&bytes).unwrap();
        let [upper, lower] = dynamic.to_be_bytes();
        let body = attribute(&[0x2a, 0xba, upper, lower, 0, 0, 0xb0], 1, 1);
        let registers = Registers::new(1, 1, 1).unwrap();
        let mut frame = Stack::default();
        frame.seed_local(0, receiver, None).unwrap();
        let mut output = Vec::new();
        let mut issues = Vec::new();
        for item in Code::parse(&body).unwrap().instructions() {
            if let Err(error) = apply(
                &class,
                item.unwrap(),
                &mut frame,
                registers,
                Some(&mut output),
            ) {
                assert!(error.retains_stack());
                issues.push(error);
            }
        }
        let capture = &output[1];
        assert_eq!(capture.args.len(), 1);
        assert_eq!(capture.args[0].reg_num(), output[0].result_reg());
        assert_eq!(output[2].args[0].reg_num(), capture.result_reg());
        if supported {
            assert!(issues.is_empty());
            assert_eq!(capture.insn_type, InsnType::Invoke);
            let site = capture.payload.lambda.as_ref().unwrap();
            assert!(!site.implementation_static);
            assert_eq!(site.implementation.name, "mix");
            assert_eq!(site.implementation.owner, ArgType::object("fixture/Ops"));
            assert_eq!(site.sam_descriptor.parameters, [ArgType::LONG]);
        } else {
            assert_eq!(issues.len(), 1);
            assert_eq!(capture.insn_type, InsnType::Unavailable);
        }
    }
}

#[test]
fn unbound_method_reference_takes_receiver_from_the_original_sam() {
    for (sam, accepted) in [
        (b"(Lfixture/Ops;J)I".as_slice(), true),
        (b"(Ljava/lang/Object;J)I".as_slice(), false),
    ] {
        let mut pool = Pool::default();
        let bootstrap = pool.method(
            b"java/lang/invoke/LambdaMetafactory",
            b"metafactory",
            b"(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/invoke/MethodType;Ljava/lang/invoke/MethodType;Ljava/lang/invoke/MethodHandle;Ljava/lang/invoke/MethodType;)Ljava/lang/invoke/CallSite;",
        );
        let handle = pool.handle(6, bootstrap);
        let method = pool.method(b"fixture/Ops", b"mix", b"(J)I");
        let target = pool.handle(5, method);
        let signature = pool.utf8(sam);
        let sam_type = pool.words(16, &[signature]);
        let declared = pool.utf8(b"mix");
        let declared_descriptor = pool.utf8(b"(J)I");
        let dynamic = pool.dynamic(b"apply", b"()Lfixture/ReceiverAndLongToInt;");
        let bytes = pool.finish(
            handle,
            &[sam_type, target, sam_type],
            &[[0x0101, declared, declared_descriptor, 0]],
        );
        let class = ClassFile::parse(&bytes).unwrap();
        let [upper, lower] = dynamic.to_be_bytes();
        let body = attribute(&[0xba, upper, lower, 0, 0, 0xb0], 1, 0);
        let code = Code::parse(&body).unwrap();
        let registers = Registers::new(0, 1, 0).unwrap();
        let mut frame = Stack::default();
        let mut output = Vec::new();
        let result = apply(
            &class,
            code.instructions().next().unwrap().unwrap(),
            &mut frame,
            registers,
            Some(&mut output),
        );
        assert_eq!(result.is_ok(), accepted);
        assert!(output[0].args.is_empty());
        if accepted {
            let site = output[0].payload.lambda.as_ref().unwrap();
            assert_eq!(
                site.sam_descriptor.parameters,
                [ArgType::object("fixture/Ops"), ArgType::LONG]
            );
            assert_eq!(site.implementation.descriptor.parameters, [ArgType::LONG]);
        } else {
            assert_eq!(output[0].insn_type, InsnType::Unavailable);
        }
    }
}
