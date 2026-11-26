use std::collections::{BTreeMap, BTreeSet};

use super::{
    JavaAssignOp, JavaAstRewriter, JavaAstTransform, JavaBinaryOp, JavaExpr, JavaIdentifier,
    JavaLiteral, JavaMethodBody, JavaPrimitiveType, JavaStmt, JavaType,
};

#[derive(Debug, Default)]
pub struct DefiniteAssignment;

impl JavaAstTransform for DefiniteAssignment {
    type Error = super::JavaStructuralError;

    fn apply(&mut self, body: &mut JavaMethodBody) -> Result<bool, Self::Error> {
        let declarations = Declarations::collect(&body.root);
        if declarations.is_empty() {
            return Ok(false);
        }

        let mut analysis = AssignmentAnalysis::new(declarations);
        analysis.analyze(&body.root);
        let mut changed = analysis.initialize(&mut body.root);
        let mut known = KnownValues::default();
        changed |= known.rewrite(&mut body.root).changed;
        Ok(changed)
    }
}

#[derive(Default)]
struct KnownValues {
    values: BTreeMap<JavaIdentifier, JavaLiteral>,
}

#[derive(Default)]
struct RewriteResult {
    changed: bool,
    completes: bool,
}

impl KnownValues {
    fn rewrite(&mut self, statement: &mut JavaStmt) -> RewriteResult {
        match statement {
            JavaStmt::RecoveryFailure(_) => {
                self.values.clear();
                Self::complete(false)
            }
            JavaStmt::Empty => Self::complete(false),
            JavaStmt::Block(statements) => self.sequence(statements),
            JavaStmt::Variable { name, value, .. } => {
                self.values.remove(name);
                if let Some(value) = value {
                    self.invalidate(value);
                    if let JavaExpr::Literal(literal) = value {
                        self.values.insert(name.clone(), literal.clone());
                    }
                }
                Self::complete(false)
            }
            JavaStmt::Assign { target, op, value } => {
                self.invalidate(value);
                self.invalidate(target);
                let JavaExpr::Name(name) = target else {
                    return Self::complete(false);
                };
                let literal = match (op, value) {
                    (JavaAssignOp::Assign, JavaExpr::Literal(literal)) => Some(literal.clone()),
                    _ => None,
                };
                if literal
                    .as_ref()
                    .is_some_and(|literal| self.values.get(name) == Some(literal))
                {
                    *statement = JavaStmt::Empty;
                    return Self::complete(true);
                }
                self.values.remove(name);
                if let Some(literal) = literal {
                    self.values.insert(name.clone(), literal);
                }
                Self::complete(false)
            }
            JavaStmt::Expression(expression) => {
                self.invalidate(expression);
                Self::complete(false)
            }
            JavaStmt::ConstructorInvocation { args, .. } => {
                for argument in args {
                    self.invalidate(argument);
                }
                Self::complete(false)
            }
            JavaStmt::If {
                condition,
                then_stmt,
                else_stmt,
            } => {
                self.invalidate(condition);
                let (when_true, true_values) = self.branch(then_stmt);
                let when_false = else_stmt
                    .as_deref_mut()
                    .map(|statement| self.rewrite(statement))
                    .unwrap_or_else(|| Self::complete(false));
                self.values = Self::join(
                    (when_true.completes, true_values),
                    (when_false.completes, std::mem::take(&mut self.values)),
                );
                RewriteResult {
                    changed: when_true.changed || when_false.changed,
                    completes: when_true.completes || when_false.completes,
                }
            }
            JavaStmt::Try {
                body,
                catches,
                finally,
            } => {
                let body_result = self.branch(body).0;
                let mut changed = body_result.changed;
                let mut completes = body_result.completes;
                for catch in catches {
                    let result = self.branch(&mut catch.body).0;
                    changed |= result.changed;
                    completes |= result.completes;
                }
                if let Some(finally) = finally {
                    let result = self.rewrite(finally);
                    changed |= result.changed;
                    completes &= result.completes;
                }
                self.values.clear();
                RewriteResult { changed, completes }
            }
            JavaStmt::Synchronized { lock, body } => {
                self.invalidate(lock);
                self.rewrite(body)
            }
            JavaStmt::Labeled { body, .. } => {
                let result = self.rewrite(body);
                self.values.clear();
                result
            }
            JavaStmt::While {
                condition, body, ..
            } => {
                self.forget_statement_writes(body);
                self.invalidate(condition);
                let result = self.rewrite(body);
                self.values.clear();
                RewriteResult {
                    changed: result.changed,
                    completes: true,
                }
            }
            JavaStmt::DoWhile { body, .. } => {
                self.forget_statement_writes(body);
                let result = self.rewrite(body);
                self.values.clear();
                RewriteResult {
                    changed: result.changed,
                    completes: true,
                }
            }
            JavaStmt::For {
                init,
                condition,
                update,
                body,
                ..
            } => {
                let mut result = self.sequence(init);
                self.forget_statement_writes(body);
                self.forget_expression_writes(update);
                if let Some(condition) = condition {
                    self.invalidate(condition);
                }
                let body_result = self.rewrite(body);
                result.changed |= body_result.changed;
                self.values.clear();
                result.completes = true;
                result
            }
            JavaStmt::ForEach { iterable, body, .. } => {
                self.forget_statement_writes(body);
                self.invalidate(iterable);
                let result = self.rewrite(body);
                self.values.clear();
                RewriteResult {
                    changed: result.changed,
                    completes: true,
                }
            }
            JavaStmt::Switch {
                selector, cases, ..
            } => {
                self.invalidate(selector);
                let mut changed = false;
                for case in cases {
                    let mut branch = Self {
                        values: self.values.clone(),
                    };
                    changed |= branch.sequence(&mut case.body).changed;
                }
                self.values.clear();
                RewriteResult {
                    changed,
                    completes: true,
                }
            }
            JavaStmt::Return(value) => {
                if let Some(value) = value {
                    self.invalidate(value);
                }
                Self::terminal()
            }
            JavaStmt::Throw(value) => {
                self.invalidate(value);
                Self::terminal()
            }
            JavaStmt::Break(_) | JavaStmt::Continue(_) => Self::terminal(),
        }
    }

    fn sequence(&mut self, statements: &mut Vec<JavaStmt>) -> RewriteResult {
        let mut result = Self::complete(false);
        for statement in statements.iter_mut() {
            if !result.completes {
                break;
            }
            let next = self.rewrite(statement);
            result.changed |= next.changed;
            result.completes = next.completes;
        }
        if result.changed {
            statements.retain(|statement| !matches!(statement, JavaStmt::Empty));
        }
        result
    }

    fn branch(
        &self,
        statement: &mut JavaStmt,
    ) -> (RewriteResult, BTreeMap<JavaIdentifier, JavaLiteral>) {
        let mut branch = Self {
            values: self.values.clone(),
        };
        let result = branch.rewrite(statement);
        (result, branch.values)
    }

    fn join(
        left: (bool, BTreeMap<JavaIdentifier, JavaLiteral>),
        right: (bool, BTreeMap<JavaIdentifier, JavaLiteral>),
    ) -> BTreeMap<JavaIdentifier, JavaLiteral> {
        match (left.0, right.0) {
            (true, false) => left.1,
            (false, true) => right.1,
            (false, false) => BTreeMap::new(),
            (true, true) => {
                let mut joined = left.1;
                joined.retain(|name, value| right.1.get(name) == Some(value));
                joined
            }
        }
    }

    fn invalidate(&mut self, expression: &JavaExpr) {
        if self.values.is_empty() {
            return;
        }
        let mut writes = ExpressionWrites::default();
        writes.rewrite_expression(expression.clone());
        for name in writes.names {
            self.values.remove(&name);
        }
    }

    fn forget_statement_writes(&mut self, statement: &JavaStmt) {
        if self.values.is_empty() {
            return;
        }
        let mut writes = StatementWrites::default();
        writes.collect(statement);
        self.forget_names(writes.names);
    }

    fn forget_expression_writes(&mut self, expressions: &[JavaExpr]) {
        if self.values.is_empty() {
            return;
        }
        let mut writes = ExpressionWrites::default();
        for expression in expressions {
            writes.rewrite_expression(expression.clone());
        }
        self.forget_names(writes.names);
    }

    fn forget_names(&mut self, names: BTreeSet<JavaIdentifier>) {
        for name in names {
            self.values.remove(&name);
        }
    }

    fn complete(changed: bool) -> RewriteResult {
        RewriteResult {
            changed,
            completes: true,
        }
    }

    fn terminal() -> RewriteResult {
        RewriteResult {
            changed: false,
            completes: false,
        }
    }
}

#[derive(Default)]
struct ExpressionWrites {
    names: BTreeSet<JavaIdentifier>,
}

impl JavaAstRewriter for ExpressionWrites {
    fn finish_expression(&mut self, expression: JavaExpr) -> JavaExpr {
        match &expression {
            JavaExpr::Assignment { target, .. } | JavaExpr::Update { target, .. } => {
                if let JavaExpr::Name(name) = target.as_ref() {
                    self.names.insert(name.clone());
                }
            }
            _ => {}
        }
        expression
    }
}

#[derive(Default)]
struct StatementWrites {
    names: BTreeSet<JavaIdentifier>,
}

impl StatementWrites {
    fn collect(&mut self, statement: &JavaStmt) {
        self.rewrite_statement(statement.clone());
    }

    fn record_target(&mut self, target: &JavaExpr) {
        if let JavaExpr::Name(name) = target {
            self.names.insert(name.clone());
        }
    }
}

impl JavaAstRewriter for StatementWrites {
    fn rewrite_nested_functions(&self) -> bool {
        false
    }

    fn finish_expression(&mut self, expression: JavaExpr) -> JavaExpr {
        if let JavaExpr::Assignment { target, .. } | JavaExpr::Update { target, .. } = &expression {
            self.record_target(target);
        }
        expression
    }

    fn finish_statement(&mut self, statement: JavaStmt) -> JavaStmt {
        match &statement {
            JavaStmt::Variable { name, .. } | JavaStmt::ForEach { variable: name, .. } => {
                self.names.insert(name.clone());
            }
            JavaStmt::Assign { target, .. } => self.record_target(target),
            JavaStmt::Empty
            | JavaStmt::RecoveryFailure(_)
            | JavaStmt::Block(_)
            | JavaStmt::Labeled { .. }
            | JavaStmt::Expression(_)
            | JavaStmt::ConstructorInvocation { .. }
            | JavaStmt::If { .. }
            | JavaStmt::While { .. }
            | JavaStmt::DoWhile { .. }
            | JavaStmt::For { .. }
            | JavaStmt::Switch { .. }
            | JavaStmt::Try { .. }
            | JavaStmt::Synchronized { .. }
            | JavaStmt::Return(_)
            | JavaStmt::Throw(_)
            | JavaStmt::Break(_)
            | JavaStmt::Continue(_) => {}
        }
        statement
    }
}

#[derive(Debug, Default)]
struct Declarations(BTreeMap<JavaIdentifier, usize>);

impl Declarations {
    fn collect(root: &JavaStmt) -> Self {
        let mut declarations = Self::default();
        let mut pending = vec![root];
        while let Some(statement) = pending.pop() {
            match statement {
                JavaStmt::Variable {
                    name, value: None, ..
                } => {
                    let index = declarations.0.len();
                    declarations.0.entry(name.clone()).or_insert(index);
                }
                JavaStmt::Block(children) => pending.extend(children),
                JavaStmt::Labeled { body, .. }
                | JavaStmt::While { body, .. }
                | JavaStmt::DoWhile { body, .. }
                | JavaStmt::For { body, .. }
                | JavaStmt::ForEach { body, .. }
                | JavaStmt::Synchronized { body, .. } => pending.push(body),
                JavaStmt::If {
                    then_stmt,
                    else_stmt,
                    ..
                } => {
                    pending.push(then_stmt);
                    if let Some(else_stmt) = else_stmt {
                        pending.push(else_stmt);
                    }
                }
                JavaStmt::Switch { cases, .. } => {
                    pending.extend(cases.iter().flat_map(|case| case.body.iter()));
                }
                JavaStmt::Try {
                    body,
                    catches,
                    finally,
                } => {
                    pending.push(body);
                    pending.extend(catches.iter().map(|catch| &catch.body));
                    if let Some(finally) = finally {
                        pending.push(finally);
                    }
                }
                JavaStmt::Empty
                | JavaStmt::RecoveryFailure(_)
                | JavaStmt::Expression(_)
                | JavaStmt::ConstructorInvocation { .. }
                | JavaStmt::Assign { .. }
                | JavaStmt::Return(_)
                | JavaStmt::Throw(_)
                | JavaStmt::Break(_)
                | JavaStmt::Continue(_)
                | JavaStmt::Variable { .. } => {}
            }
        }
        declarations
    }

    fn index(&self, name: &JavaIdentifier) -> Option<usize> {
        self.0.get(name).copied()
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Only declarations with a missing initializer can require a default value.
/// Dense indices let branches copy and intersect words instead of identifier trees.
#[derive(Clone, Debug, Default)]
struct Assigned {
    words: smallvec::SmallVec<[u64; 1]>,
}

impl Assigned {
    fn set(&mut self, candidate: Option<usize>, present: bool) {
        let Some(index) = candidate else { return };
        let word = index / 64;
        let mask = 1u64 << (index % 64);
        if present {
            if word >= self.words.len() {
                self.words.resize(word + 1, 0);
            }
            self.words[word] |= mask;
        } else if let Some(bits) = self.words.get_mut(word) {
            *bits &= !mask;
        }
    }

    fn contains(&self, index: usize) -> bool {
        self.words
            .get(index / 64)
            .is_some_and(|bits| bits & (1u64 << (index % 64)) != 0)
    }

    fn intersect(&mut self, other: &Self) {
        self.words.truncate(other.words.len());
        for (bits, other) in self.words.iter_mut().zip(&other.words) {
            *bits &= other;
        }
    }
}

#[derive(Debug, Clone)]
struct Flow {
    assigned: Assigned,
    completes: bool,
}

impl Default for Flow {
    fn default() -> Self {
        Self {
            assigned: Assigned::default(),
            completes: true,
        }
    }
}

impl Flow {
    fn normal_join(flows: impl IntoIterator<Item = Self>) -> Self {
        let mut normal = flows.into_iter().filter(|flow| flow.completes);
        let Some(mut joined) = normal.next() else {
            return Self {
                assigned: Assigned::default(),
                completes: false,
            };
        };
        for flow in normal {
            joined.assigned.intersect(&flow.assigned);
        }
        joined
    }
}

#[derive(Debug)]
struct AssignmentAnalysis {
    declarations: Declarations,
    required: BTreeSet<JavaIdentifier>,
}

impl AssignmentAnalysis {
    fn new(declarations: Declarations) -> Self {
        Self {
            declarations,
            required: BTreeSet::new(),
        }
    }

    fn analyze(&mut self, root: &JavaStmt) {
        self.statement(root, Flow::default());
    }

    fn statement(&mut self, statement: &JavaStmt, mut flow: Flow) -> Flow {
        match statement {
            JavaStmt::RecoveryFailure(_) => {
                flow.assigned.words.clear();
                flow
            }
            JavaStmt::Empty => flow,
            JavaStmt::Block(statements) => self.sequence(statements, flow),
            JavaStmt::Labeled { body, .. } => {
                let body_flow = self.statement(body, flow.clone());
                Flow::normal_join([flow, body_flow])
            }
            JavaStmt::Variable { name, value, .. } => {
                flow.assigned.set(self.declarations.index(name), false);
                if let Some(value) = value {
                    self.expression(value, &mut flow.assigned);
                    flow.assigned.set(self.declarations.index(name), true);
                }
                flow
            }
            JavaStmt::Expression(expression) => {
                self.expression(expression, &mut flow.assigned);
                flow
            }
            JavaStmt::ConstructorInvocation { args, .. } => {
                self.expressions(args, &mut flow.assigned);
                flow
            }
            JavaStmt::Assign { target, op, value } => {
                self.assignment(target, *op, value, &mut flow.assigned);
                flow
            }
            JavaStmt::If {
                condition,
                then_stmt,
                else_stmt,
            } => {
                self.expression(condition, &mut flow.assigned);
                let when_true = self.statement(then_stmt, flow.clone());
                let when_false = match else_stmt {
                    Some(statement) => self.statement(statement, flow),
                    None => flow,
                };
                Flow::normal_join([when_true, when_false])
            }
            JavaStmt::While {
                condition, body, ..
            } => {
                self.expression(condition, &mut flow.assigned);
                self.statement(body, flow.clone());
                flow
            }
            JavaStmt::DoWhile {
                body, condition, ..
            } => {
                let body_flow = self.statement(body, flow.clone());
                let mut condition_flow = if body_flow.completes {
                    body_flow
                } else {
                    flow.clone()
                };
                self.expression(condition, &mut condition_flow.assigned);
                Flow::normal_join([flow, condition_flow])
            }
            JavaStmt::For {
                init,
                condition,
                update,
                body,
                ..
            } => {
                flow = self.sequence(init, flow);
                if !flow.completes {
                    return flow;
                }
                if let Some(condition) = condition {
                    self.expression(condition, &mut flow.assigned);
                }
                let mut iteration = self.statement(body, flow.clone());
                if !iteration.completes {
                    iteration = flow.clone();
                }
                self.expressions(update, &mut iteration.assigned);
                Flow::normal_join([flow, iteration])
            }
            JavaStmt::ForEach {
                variable,
                iterable,
                body,
                ..
            } => {
                self.expression(iterable, &mut flow.assigned);
                let mut iteration = flow.clone();
                iteration
                    .assigned
                    .set(self.declarations.index(variable), true);
                self.statement(body, iteration);
                flow
            }
            JavaStmt::Switch {
                selector, cases, ..
            } => {
                self.expression(selector, &mut flow.assigned);
                let branches = std::iter::once(flow.clone()).chain(
                    cases
                        .iter()
                        .map(|case| self.sequence(&case.body, flow.clone())),
                );
                Flow::normal_join(branches)
            }
            JavaStmt::Try {
                body,
                catches,
                finally,
            } => {
                let body_flow = self.statement(body, flow.clone());
                let branches = std::iter::once(body_flow).chain(catches.iter().map(|catch| {
                    let mut catch_flow = flow.clone();
                    catch_flow
                        .assigned
                        .set(self.declarations.index(&catch.variable), true);
                    self.statement(&catch.body, catch_flow)
                }));
                let joined = Flow::normal_join(branches);
                let Some(finally) = finally else {
                    return joined;
                };
                // A finally block is also reached by exceptional edges from
                // any point in the protected body or a handler.
                self.statement(finally, flow.clone());
                let joined_completes = joined.completes;
                let finally_flow = self.statement(finally, joined);
                if joined_completes && finally_flow.completes {
                    finally_flow
                } else {
                    Flow {
                        assigned: Assigned::default(),
                        completes: false,
                    }
                }
            }
            JavaStmt::Synchronized { lock, body } => {
                self.expression(lock, &mut flow.assigned);
                self.statement(body, flow)
            }
            JavaStmt::Return(value) => {
                if let Some(value) = value {
                    self.expression(value, &mut flow.assigned);
                }
                flow.completes = false;
                flow
            }
            JavaStmt::Throw(value) => {
                self.expression(value, &mut flow.assigned);
                flow.completes = false;
                flow
            }
            JavaStmt::Break(_) | JavaStmt::Continue(_) => {
                flow.completes = false;
                flow
            }
        }
    }

    fn sequence(&mut self, statements: &[JavaStmt], mut flow: Flow) -> Flow {
        for statement in statements {
            if !flow.completes {
                break;
            }
            flow = self.statement(statement, flow);
        }
        flow
    }

    fn expressions(&mut self, expressions: &[JavaExpr], assigned: &mut Assigned) {
        for expression in expressions {
            self.expression(expression, assigned);
        }
    }

    fn expression(&mut self, expression: &JavaExpr, assigned: &mut Assigned) {
        match expression {
            JavaExpr::Allocate(_) => {}
            JavaExpr::DirectInit { receiver, args, .. } => {
                self.expression(receiver, assigned);
                self.expressions(args, assigned);
            }
            JavaExpr::This
            | JavaExpr::QualifiedThis(_)
            | JavaExpr::Super
            | JavaExpr::Literal(_)
            | JavaExpr::ClassLiteral(_)
            | JavaExpr::StaticField { .. } => {}
            JavaExpr::Name(name) => self.read(name, assigned),
            JavaExpr::Field { owner, .. } => self.expression(owner, assigned),
            JavaExpr::ArrayAccess { array, index } => {
                self.expression(array, assigned);
                self.expression(index, assigned);
            }
            JavaExpr::Call { receiver, args, .. } => {
                if let Some(receiver) = receiver {
                    self.expression(receiver, assigned);
                }
                self.expressions(args, assigned);
            }
            JavaExpr::MethodReference { receiver, .. } => {
                self.expression(receiver, assigned);
            }
            JavaExpr::Lambda { body, .. } => {
                let mut lambda_assigned = assigned.clone();
                self.expression(body, &mut lambda_assigned);
            }
            JavaExpr::BlockLambda { body, .. } => {
                self.statement(
                    body,
                    Flow {
                        assigned: assigned.clone(),
                        completes: true,
                    },
                );
            }
            JavaExpr::New {
                enclosing, args, ..
            } => {
                if let Some(enclosing) = enclosing {
                    self.expression(enclosing, assigned);
                }
                self.expressions(args, assigned);
            }
            JavaExpr::NewArray {
                dimensions,
                initializer,
                ..
            } => {
                self.expressions(dimensions, assigned);
                self.expressions(initializer, assigned);
            }
            JavaExpr::Unary { operand, .. }
            | JavaExpr::Cast { value: operand, .. }
            | JavaExpr::InstanceOf { value: operand, .. } => self.expression(operand, assigned),
            JavaExpr::Update { target, .. } => {
                match target.as_ref() {
                    JavaExpr::Name(name) => self.read(name, assigned),
                    target => self.expression(target, assigned),
                }
                if let JavaExpr::Name(name) = target.as_ref() {
                    assigned.set(self.declarations.index(name), true);
                }
            }
            JavaExpr::Binary { left, op, right } => {
                self.expression(left, assigned);
                if matches!(op, JavaBinaryOp::LogicalAnd | JavaBinaryOp::LogicalOr) {
                    // The right operand can add assignments only on the path that evaluates it.
                    self.expression(right, &mut assigned.clone());
                } else {
                    self.expression(right, assigned);
                }
            }
            JavaExpr::Conditional {
                condition,
                when_true,
                when_false,
            } => {
                self.expression(condition, assigned);
                let mut true_state = assigned.clone();
                self.expression(when_true, &mut true_state);
                self.expression(when_false, assigned);
                assigned.intersect(&true_state);
            }
            JavaExpr::Assignment { target, op, value } => {
                self.assignment(target, *op, value, assigned);
            }
        }
    }

    fn assignment(
        &mut self,
        target: &JavaExpr,
        op: JavaAssignOp,
        value: &JavaExpr,
        assigned: &mut Assigned,
    ) {
        match target {
            JavaExpr::Name(name) => {
                if op != JavaAssignOp::Assign {
                    self.read(name, assigned);
                }
            }
            target => self.expression(target, assigned),
        }
        self.expression(value, assigned);
        if let JavaExpr::Name(name) = target {
            assigned.set(self.declarations.index(name), true);
        }
    }

    fn read(&mut self, name: &JavaIdentifier, assigned: &Assigned) {
        if self
            .declarations
            .index(name)
            .is_some_and(|index| !assigned.contains(index))
        {
            self.required.insert(name.clone());
        }
    }

    fn initialize(&self, root: &mut JavaStmt) -> bool {
        let mut changed = false;
        Self::initialize_statement(root, &self.required, &mut changed);
        changed
    }

    fn initialize_statement(
        statement: &mut JavaStmt,
        required: &BTreeSet<JavaIdentifier>,
        changed: &mut bool,
    ) {
        match statement {
            JavaStmt::Variable {
                ty,
                name,
                value: value @ None,
            } if required.contains(name) => {
                *value = Some(Self::default_value(ty));
                *changed = true;
            }
            JavaStmt::Block(children) => {
                for child in children {
                    Self::initialize_statement(child, required, changed);
                }
            }
            JavaStmt::Labeled { body, .. }
            | JavaStmt::While { body, .. }
            | JavaStmt::DoWhile { body, .. }
            | JavaStmt::For { body, .. }
            | JavaStmt::ForEach { body, .. }
            | JavaStmt::Synchronized { body, .. } => {
                Self::initialize_statement(body, required, changed);
            }
            JavaStmt::If {
                then_stmt,
                else_stmt,
                ..
            } => {
                Self::initialize_statement(then_stmt, required, changed);
                if let Some(else_stmt) = else_stmt {
                    Self::initialize_statement(else_stmt, required, changed);
                }
            }
            JavaStmt::Switch { cases, .. } => {
                for case in cases {
                    for child in &mut case.body {
                        Self::initialize_statement(child, required, changed);
                    }
                }
            }
            JavaStmt::Try {
                body,
                catches,
                finally,
            } => {
                Self::initialize_statement(body, required, changed);
                for catch in catches {
                    Self::initialize_statement(&mut catch.body, required, changed);
                }
                if let Some(finally) = finally {
                    Self::initialize_statement(finally, required, changed);
                }
            }
            JavaStmt::Empty
            | JavaStmt::RecoveryFailure(_)
            | JavaStmt::Variable { .. }
            | JavaStmt::Expression(_)
            | JavaStmt::ConstructorInvocation { .. }
            | JavaStmt::Assign { .. }
            | JavaStmt::Return(_)
            | JavaStmt::Throw(_)
            | JavaStmt::Break(_)
            | JavaStmt::Continue(_) => {}
        }
    }

    fn default_value(ty: &JavaType) -> JavaExpr {
        let literal = match ty {
            JavaType::Primitive(JavaPrimitiveType::Boolean) => JavaLiteral::Boolean(false),
            JavaType::Primitive(JavaPrimitiveType::Long) => JavaLiteral::Long(0),
            JavaType::Primitive(JavaPrimitiveType::Float) => JavaLiteral::Float(0.0),
            JavaType::Primitive(JavaPrimitiveType::Double) => JavaLiteral::Double(0.0),
            JavaType::Primitive(JavaPrimitiveType::Char) => JavaLiteral::Character(0),
            JavaType::Primitive(
                JavaPrimitiveType::Byte | JavaPrimitiveType::Short | JavaPrimitiveType::Int,
            ) => JavaLiteral::Integer(0),
            JavaType::Primitive(JavaPrimitiveType::Void)
            | JavaType::Class(_)
            | JavaType::Variable(_)
            | JavaType::Array(_) => JavaLiteral::Null,
        };
        JavaExpr::Literal(literal)
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/definite_assignment.rs"]
mod tests;
