extern g_idcutil_logfile;
static LogInit()
{
  g_idcutil_logfile = fopen("idaout.txt", "w+");
  if (g_idcutil_logfile == 0)
    return 0;
  return 1;
}

static main()
{
    LogInit();
    fprintf(g_idcutil_logfile, "%s", "{\"functions\":[");
    msg("%s", "{\"functions\":[");
    auto ea, x;  for ( ea=NextFunction(0); ea != BADADDR; ea=NextFunction(ea) )
    {
        fprintf(g_idcutil_logfile, "{\"name\": \"%s\" ,  \"start\" : %ld, \"end\": %ld}\n", GetFunctionName(ea), ea - get_imagebase(), find_func_end(ea) - get_imagebase());
        msg("{\"name\": \"%s\" ,  \"start\" : %ld, \"end\": %ld}", GetFunctionName(ea), ea - get_imagebase(), find_func_end(ea) - get_imagebase());
        if (NextFunction(ea) != BADADDR) {
            msg("%s", ",\n");
            fprintf(g_idcutil_logfile, "%s", ",\n");
        }
    }

    fprintf(g_idcutil_logfile, "%s", "]}");
    msg("%s", "]}\n");
    
    msg("Saved to idaout.txt");
    fclose(g_idcutil_logfile);
}