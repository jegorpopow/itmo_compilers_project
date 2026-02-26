type kilometers is real;
type miles is real; 

routine meow(value:  kilometers) : miles => value;

routine main() is
  var dist : kilometers is 10.0;
  var result : miles is meow(dist);
  print result;
end;