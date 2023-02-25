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
  uint64_t loop_ids = 0;


  loop_counter_pass(gcc::context *ctx):
    gimple_opt_pass(loop_counter_pass_data, ctx) {}

  bool insert_counter (class loop *loop);

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

    return change ? (TODO_update_ssa | TODO_cleanup_cfg) : 0;
  }

};

bool
loop_counter_pass::insert_counter (class loop *loop)
{
  basic_block preheader = loop_preheader_edge(loop)->src;
  if (!preheader || !loop->header)
    return false;

  tree loopid = build_int_cst(uint64_type_node, this->loop_ids++);
  gcall *stmt = gimple_build_call(libclrt_preheader_fun, 1, loopid);
  gimple_stmt_iterator gsi = gsi_start_bb (preheader);
  gsi_insert_before(&gsi, stmt, GSI_NEW_STMT);

  tree callres = make_temp_ssa_name(uint64_type_node, NULL, "continue");
  gcall *call = gimple_build_call(libclrt_header_fun, 1, loopid);
  gimple_call_set_lhs(call, callres);
  gsi = gsi_start_bb(loop->header);
  gsi_insert_before(&gsi, call, GSI_CONTINUE_LINKING);

  tree zero = build_zero_cst(uint64_type_node);
  edge e = split_block(loop->header, gsi_stmt (gsi));



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

