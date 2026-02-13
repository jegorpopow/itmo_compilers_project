routine abs(value : real) is 
  if value > 0.0 then 
    value := 0.0 - value;
  end;
  return value;
end;

routine main(a : int) {
    if a % 2 == 0 then 
      var dummy is 0;
      print 0;
      print dummy;
    else 
      print 1;
    end;
}