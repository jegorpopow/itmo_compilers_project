routine echo (data : integer) : integer is
  print data;
  return data;
end;

routine main() is
  print echo(42);
end;
