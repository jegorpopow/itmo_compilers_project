routine collatz(n : int) : int is
  var steps is 0;

  while n != 1 loop 
    if n % 2 == 0 then 
      n := n / 2;
    else 
      n := 3 * n + 1;
    end;
    step := step + 1;
  end; 

  return steps;
end.
