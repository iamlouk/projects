#include <cstdlib>
#include <cstdio>
#include <cstdint>
#include <cassert>

#include "config.h"
#include "system.h"
#include "coretypes.h"
#include "toplev.h"
#include "function.h"
#include "gcc-plugin.h"
#include "plugin-version.h"
#include "tree-core.h"
#include "tree.h"
#include "gimple.h"
#include "gimple-iterator.h"
#include "basic-block.h"
#include "tree-cfg.h"
#include "tree-pass.h"
#include "dumpfile.h"
#include "cfgloop.h"
#include "cfghooks.h"
#include "ssa.h"
#include "tree-scalar-evolution.h"
#include "tree-ssa-loop-niter.h"
#include "stringpool.h"
#include "cgraph.h"
#include "context.h"
#include "diagnostic.h"
#include "langhooks.h"
#include "print-tree.h"
#include "dominance.h"
#include "tree-ssanames.h"

/* GCC will look for this magic symbol when loading
   the plugin and fail if it is not found:  */
int plugin_is_GPL_compatible = 1;

static const pass_data loop_counter_pass_data =
{
  GIMPLE_PASS,
  "loop-counter",
  OPTGROUP_LOOP,
  TV_NONE,
  ( PROP_cfg | PROP_ssa | PROP_gimple ),
  0,
  0,
  0,
  0
};

static tree libclrt_header_fun = NULL_TREE;
static tree libclrt_preheader_fun = NULL_TREE;

/*
 * For some reason, add_builtin_function cannot
 * be called directly when the plugin is loaded,
 * but doing it in the pass execute function
 * seams wrong as well.
 */
static void
plugin_setup (void *gcc_data, void *user_data)
{
  (void) gcc_data;
  (void) user_data;

  tree fntype = build_function_type_list(uint64_type_node,
					 uint64_type_node, NULL);
  libclrt_header_fun = add_builtin_function("__gcclc_loop_header",
					    fntype, 0, NOT_BUILT_IN,
					    NULL, NULL);

  fntype = build_function_type_list(void_type_node,
				    uint64_type_node, NULL);
  libclrt_preheader_fun = add_builtin_function("__gcclc_loop_preheader",
					       fntype, 0, NOT_BUILT_IN,
					       NULL, NULL);
}


struct loop_counter_pass: gimple_opt_pass
{
  /* TODO: Use debug information like source file name, function name and line numbers! */
  uint64_t loop_ids = 0;

  loop_counter_pass(gcc::context *ctx):
    gimple_opt_pass(loop_counter_pass_data, ctx) {}

  bool insert_counter (class loop *loop);

  /* Actually optional, but if I add a enable/disable pragma this will become useful: */
  virtual bool gate(function *fun) final override
  {
    (void) fun;
    return true;
  }

  virtual unsigned int execute(function *fun) final override
  {
    assert (libclrt_header_fun && libclrt_preheader_fun);
    bool change = false;
    bool in_loop_pipeline = scev_initialized_p ();
    if (!in_loop_pipeline)
      {
	/* Loops need to be in normal for so that we can
	 * access the preheader and so on.
	 */
	loop_optimizer_init (LOOPS_NORMAL);
	scev_initialize ();
      }

    for (class loop* loop: loops_list (fun, 0))
      change |= insert_counter (loop);

    if (!in_loop_pipeline)
      {
	scev_finalize ();
	loop_optimizer_finalize ();
      }

    /* The way this is currently implemented, I do not think
     * that cleanup_cfg is really needed.
     */
    return change ? (TODO_update_ssa | TODO_cleanup_cfg) : 0;
  }

};

bool
loop_counter_pass::insert_counter (class loop *loop)
{
  basic_block preheader = loop_preheader_edge (loop)->src;
  if (!preheader || !loop->header || EDGE_COUNT (loop->header->succs) != 2)
    return false;

  /* Build the call in the preheader: */
  tree loopid = build_int_cst (uint64_type_node, this->loop_ids++);
  gcall *stmt = gimple_build_call (libclrt_preheader_fun, 1, loopid);
  gimple_stmt_iterator gsi = gsi_start_bb (preheader);
  gsi_insert_before (&gsi, stmt, GSI_NEW_STMT);

  /* Build the call in the header: */
  tree callres = make_temp_ssa_name (uint64_type_node, NULL, "continue");
  gcall *call = gimple_build_call (libclrt_header_fun, 1, loopid);
  gimple_call_set_lhs (call, callres);
  gsi = gsi_start_bb (loop->header);
  gsi_insert_before (&gsi, call, GSI_CONTINUE_LINKING);

  gsi = gsi_last_bb(loop->header);
  gcond *cond = dyn_cast<gcond*> (gsi_stmt (gsi));
  if (!cond) /* Can only happen in infinite loops, forget about them for now. */
    return false;

  gsi_prev (&gsi); /* Lets start inserting right before the cond stmt. */

  /* `if (a CMP b) goto ...;` becomes `oldcond = a CMP b` */
  tree cond1 = make_temp_ssa_name (boolean_type_node, NULL, "origcond");
  tree expr1 = build2 (gimple_cond_code (cond), boolean_type_node,
		       gimple_cond_lhs(cond), gimple_cond_rhs(cond));
  gassign *cond1_stmt = gimple_build_assign (cond1, expr1);
  gsi_insert_after(&gsi, cond1_stmt, GSI_CONTINUE_LINKING);

  /* Check if the __gcclc_loop_header function returned something other than zero */
  tree cond2 = make_temp_ssa_name (boolean_type_node, NULL, "controlcond");
  tree expr2 = build2 (NE_EXPR, boolean_type_node, callres, build_zero_cst (uint64_type_node));
  gassign *cond2_stmt = gimple_build_assign (cond2, expr2);
  gsi_insert_after(&gsi, cond2_stmt, GSI_CONTINUE_LINKING);

  /* Create a new statement `newcond = oldcond && <__gcclc_loop_header(...) != 0>` */
  enum tree_code ccode = EDGE_SUCC (loop->header, 0)->dest == loop->latch ? BIT_AND_EXPR : BIT_IOR_EXPR;
  tree ncond = make_temp_ssa_name(boolean_type_node, NULL, "cond");
  gassign *ncond_stmt = gimple_build_assign(ncond, build2(ccode, boolean_type_node, cond1, cond2));
  gsi_insert_after(&gsi, ncond_stmt, GSI_CONTINUE_LINKING);

  /* Replace the old condition and use `if (newcond == true) goto ...;` instead. */
  gimple_cond_set_code(cond, EQ_EXPR);
  gimple_cond_set_lhs(cond, ncond);
  gimple_cond_set_rhs(cond, boolean_true_node);
  update_stmt (cond);

  debug_loop (loop, 3);
  return true;
}

extern int
plugin_init (struct plugin_name_args *plugin_info,
	     struct plugin_gcc_version *version)
{
  if (!plugin_default_version_check (version, &gcc_version))
  {
    fprintf (stderr, "GCC plugin: loop-counter is for GCC %d.%d\n",
	     GCCPLUGIN_VERSION_MAJOR, GCCPLUGIN_VERSION_MINOR);
    return EXIT_FAILURE;
  }

  const char *plugin_name = plugin_info->base_name;
  struct plugin_argument *argv = plugin_info->argv;
  for (int i = 0; i < plugin_info->argc; i++)
    {
      if (!strcmp (argv[i].key, "disable"))
	return EXIT_SUCCESS;
      else
	warning (0, "plugin %gs: unrecognized argument %gs ignored",
		 plugin_name, argv[i].key);
    }

  struct register_pass_info pass_info =
  {
    .pass = new loop_counter_pass (g),
    .reference_pass_name = "ssa",
    .ref_pass_instance_number = 1,
    .pos_op = PASS_POS_INSERT_AFTER
  };

  register_callback(plugin_name, PLUGIN_START_UNIT, &plugin_setup, pass_info.pass);
  register_callback(plugin_name, PLUGIN_PASS_MANAGER_SETUP, NULL, &pass_info);
  return EXIT_SUCCESS;
}

