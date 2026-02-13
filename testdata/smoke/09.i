routine array_length (a : array [] int) : int;
routine bubble_sort (a : array [] int, length : int);

routine sort_and_print_reversed_array(a : array [] int) is
  bubble_sort(a, )
  for elem in a ..  reversed loop 
    print elem;
  end;
end;

routine array_length (a : array [] int) : int is 
  var result is 0;
  for _ in a.. loop
    result := result + 1;
  end;
  return result;
end;

routine bubble_sort (a : array [] int, length : int) is 
  for i in 1 .. n loop
    for j in 2 .. i loop 
      if a[j - 1] > a[j] then
        var t is a[j];
        a[j] := a[j - 1];
        a[j - 1] := t;
      end;
    end; 
  end;
end.

routine countdown (n: int) is 
  for i in n .. 0 reversed loop 
    print i;
  end;
end;
