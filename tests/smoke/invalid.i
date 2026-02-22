routine extra_return() is
  return 10;
end;

routine no_return() : real is 
  extra_return();
end;

type a is real;
type b is real;

routine strange_cast() : real is
  var i : a;
  var j : b;
  i := j;
end;
