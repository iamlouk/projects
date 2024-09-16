import inspect
import ast
from typing import Any, Tuple
import typing
import ctypes
import llvmlite.binding as llvm
from llvmlite import ir


llvm.initialize()
llvm.initialize_native_target()
llvm.initialize_native_asmprinter()
target = llvm.Target.from_default_triple()
target_machine = target.create_target_machine()
execution_engine = llvm.create_mcjit_compiler(llvm.parse_assembly(""), target_machine)

def getType(name: ast.Name) -> Tuple[ir.Type, Any]:
    match str(name.id):
        case "int": return (ir.IntType(64), ctypes.c_int64)
        case "float": return (ir.DoubleType(), ctypes.c_double)
        case _: raise RuntimeError(f"unsupported type: {ast.dump(name)}")

def jit(func):
    tree = ast.parse(inspect.getsource(func)).body[0]
    if not isinstance(tree, ast.FunctionDef):
        raise RuntimeError("function definition expected!")
    print(f"@jit: function_name={tree.name}: {ast.dump(tree)}")

    if len(tree.args.kwonlyargs) > 0 or len(tree.args.posonlyargs) > 0 \
            or len(tree.args.defaults) > 0 or len(tree.args.kw_defaults) > 0 \
            or tree.returns is None or not isinstance(tree.returns, ast.Name):
        raise RuntimeError("Unsupported declarations")

    retty, retcty = getType(tree.returns)
    args: list[Tuple[str, ir.Type, Any]] = []
    for arg in tree.args.args:
        if isinstance(arg.annotation, ast.Name):
            irty, cty = getType(arg.annotation)
            args.append((str(arg.arg), irty, cty))
        elif isinstance(arg.annotation, ast.Subscript):
            val = arg.annotation.value
            if isinstance(val, ast.Name) and str(val.id) == 'list' \
                    and isinstance(arg.annotation.slice, ast.Name):
                baseirty, basecty = getType(arg.annotation.slice)
                args.append((str(arg.arg), ir.PointerType(baseirty), ctypes.POINTER(basecty)))
        else:
            raise RuntimeError("Type annotation needed/invalid!")

    fnty = ir.FunctionType(retty, map(lambda t: t[1], args))
    func_module = ir.Module(f"badjit_function_{tree.name}")
    func = ir.Function(func_module, fnty, f"{tree.name}_{tree.lineno}")
    entry_bb: ir.Block = func.append_basic_block('entry')
    builder = ir.IRBuilder(entry_bb)

    variables: dict[str, Tuple[ir.Value, ir.Type]] = {}
    for (i, (name, irty, _)) in enumerate(args):
        ptr = builder.alloca(irty, 1, name)
        builder.store(func.args[i], ptr)
        variables[name] = (ptr, irty)

    def handle_expr(b: ir.IRBuilder, e: ast.expr) -> tuple[ir.Value, ir.Type]:
        if isinstance(e, ast.Name):
            if str(e.id) not in variables:
                raise RuntimeError(f"unknown name: {e.id}")

            ptr, irty = variables[str(e.id)]
            val = b.load(ptr)
            return val, irty

        if isinstance(e, ast.Constant) and isinstance(e.value, int):
            return ir.Constant(ir.IntType(64), e.value), ir.IntType(64)

        if isinstance(e, ast.Constant) and isinstance(e.value, float):
            return ir.Constant(ir.DoubleType(), e.value), ir.DoubleType()

        if isinstance(e, ast.Call) and isinstance(e.func, ast.Name):
            if str(e.func.id) == str(tree.name):
                args: list[ir.Value] = []
                for arg in e.args:
                    val, _ = handle_expr(b, arg)
                    args.append(val)
                res = b.call(func, args)
                return (res, retty)

        if isinstance(e, ast.BinOp):
            lhs, t1 = handle_expr(b, e.left)
            rhs, t2 = handle_expr(b, e.right)
            assert t1 == t2
            match e.op:
                case _ if isinstance(e.op, ast.Add):
                    return typing.cast(ir.Value, b.add(lhs, rhs)), t1
                case _ if isinstance(e.op, ast.Sub):
                    return typing.cast(ir.Value, b.sub(lhs, rhs)), t1
                case _:
                    raise RuntimeError(f"unsupported binary operator: {ast.dump(e.op)}")

        if isinstance(e, ast.Compare):
            assert len(e.comparators) == 1 and len(e.ops) == 1
            lhs, t1 = handle_expr(b, e.left)
            rhs, t2 = handle_expr(b, e.comparators[0])
            assert t1 == t2
            match e.ops[0]:
                case gt if isinstance(gt, ast.Gt):
                    return typing.cast(ir.Value, b.icmp_signed('>', lhs, rhs)), ir.IntType(1)
                case lt if isinstance(lt, ast.Lt):
                    return typing.cast(ir.Value, b.icmp_signed('<', lhs, rhs)), ir.IntType(1)
                case op:
                    raise RuntimeError(f"unsupported comparison: {ast.dump(op)}")

        raise RuntimeError(f"unsupported expression: {ast.dump(e)}")

    def handle_assignment(b: ir.IRBuilder, name: str, val: ir.Value, ty: ir.Type):
        if name in variables:
            ptr, _ = variables[name]
            b.store(val, ptr)
        else:
            alloca_builder = ir.IRBuilder(entry_bb)
            alloca_builder.position_before(entry_bb.terminator)
            ptr = alloca_builder.alloca(ty, 1, name)
            b.store(val, ptr)
            variables[name] = (ptr, ty)

    def handle_stmt(b: ir.IRBuilder, stmt: ast.stmt) -> ir.IRBuilder:
        print(f"stmt: {ast.dump(stmt)}")

        if isinstance(stmt, ast.Return):
            if stmt.value is None:
                b.ret_void()
            else:
                val, ty = handle_expr(b, stmt.value)
                b.ret(val)

            dead_bb = func.append_basic_block('already.returned')
            b.position_at_start(dead_bb)
            return b

        if isinstance(stmt, ast.Assign):
            assert len(stmt.targets) == 1 and isinstance(stmt.targets[0], ast.Name)
            val, ty = handle_expr(b, stmt.value)
            handle_assignment(b, str(stmt.targets[0].id), val, ty)
            return b

        if isinstance(stmt, ast.While):
            assert len(stmt.orelse) == 0
            check_bb = func.append_basic_block('while.cond')
            loop_bb = func.append_basic_block('while.body')
            end_bb = func.append_basic_block('while.end')
            b.branch(check_bb)
            b.position_at_start(check_bb)
            cond, _ = handle_expr(b, stmt.test)
            b.cbranch(cond, loop_bb, end_bb)
            b.position_at_start(loop_bb)
            for substmt in stmt.body:
                b = handle_stmt(b, substmt)
            b.branch(check_bb)
            b.position_at_start(end_bb)
            return b

        if isinstance(stmt, ast.If):
            true_bb = func.append_basic_block('if.then')
            false_bb = func.append_basic_block('if.else')
            end_bb = func.append_basic_block('if.end')
            cond, _ = handle_expr(b, stmt.test)
            b.cbranch(cond, true_bb, false_bb)
            b.position_at_start(true_bb)
            for substmt in stmt.body:
                b = handle_stmt(b, substmt)
            b.branch(end_bb)
            b.position_at_start(false_bb)
            for substmt in stmt.orelse:
                b = handle_stmt(b, substmt)
            b.branch(end_bb)
            b.position_at_start(end_bb)
            return b

        if isinstance(stmt, ast.For):
            assert len(stmt.orelse) == 0
            assert isinstance(stmt.iter, ast.Call) and isinstance(stmt.iter.func, ast.Name) \
                and str(stmt.iter.func.id) == "range" and len(stmt.iter.args) == 2

            # TODO...

            assert False

        raise RuntimeError(f"unsupported statement: {ast.dump(stmt)}")

    bb = func.append_basic_block('start')
    builder.branch(bb)
    builder = ir.IRBuilder(bb)
    for stmt in tree.body:
        builder = handle_stmt(builder, stmt)
    builder.unreachable()

    print(f"{func}")

    binding_module = llvm.parse_assembly(str(func_module))
    binding_module.verify()

    pm = llvm.FunctionPassManager(binding_module)
    pmb = llvm.PassManagerBuilder()
    pmb.opt_level = 2
    pmb.populate(pm)
    pm.run(binding_module.get_function(func.name))

    execution_engine.add_module(binding_module)
    execution_engine.finalize_object()
    func_ptr = execution_engine.get_function_address(func.name)
    cfunc = ctypes.CFUNCTYPE(retcty, *map(lambda x: x[2], args))(func_ptr)
    def jitted(*args, **kwargs):
        assert len(kwargs) == 0
        return cfunc(*args)
    return jitted

