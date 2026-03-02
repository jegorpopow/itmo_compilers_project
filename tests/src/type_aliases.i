type kilometers is real;
type miles is real; 


routine miles_to_kiometers(dist : miles) => ((dist :: real) * 1.60934) :: kilometers;

routine main() is
  var dist : miles is 10.0;
  var result : kilometers is miles_to_kiometers(dist);
  print result;
end;
