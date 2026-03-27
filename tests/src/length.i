type square is record
  var length is real;
end;

routine print_length (a : array [] integer, s : square) is
  print a.length;
  print s.length;
end

routine main() is
  var a is new array[5] integer;
  var s is new square where length is 10; end;
  print_length(a, s);
end
